use crate::data::{KodiResult, Page, SubContent};
use crate::{Kodi, PathAccessData, KodiError};

use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum RecurseError {
    CalledError(PathAccessData, String),
    ThreadPanicked(PathAccessData),
    KodiCallError(PathAccessData, Arc<KodiError>),
}

#[derive(Clone)]
pub struct RecurseInfo<'a> {
    page: &'a Page,
    pub sub_content_from_parent: Option<&'a SubContent>,
    pub access: &'a PathAccessData,
    pub errors: Vec<RecurseError>,
}

impl<'a> RecurseInfo<'a> {
    pub fn get_page(&self) -> &'a Page {
        return self.page
    }

    pub fn get_sub_content_from_parent(&self) -> Option<&'a SubContent> {
        return self.sub_content_from_parent
    }

    pub fn get_access(&self) -> &'a PathAccessData {
        self.access
    }

    pub fn add_error_string(&mut self, error_message: String) {
        self.errors.push(RecurseError::CalledError(self.access.clone(), error_message));
    }

}

//TODO: put keep going here
struct SpawnNewThreadData {
    thread_nb: usize,
    effective_task: Arc<Mutex<usize>>,
    condvar: Condvar,
    is_poisoned: RwLock<bool>,
    errors: Mutex<Vec<RecurseError>>
}

impl SpawnNewThreadData {
    fn decrement_worker(&self) {
        let mut effective_lock = self.effective_task.lock().unwrap();
        *effective_lock -= 1;
        self.condvar.notify_all();
    }

    fn wait_to_spawn_child_then_increment_worker(&self) -> bool {
        let mut effective_lock = self.effective_task.lock().unwrap();
        loop {
            if *effective_lock < self.thread_nb {
                *effective_lock += 1;
                return *effective_lock == self.thread_nb;
            };
            effective_lock = self
                .condvar
                .wait_timeout(effective_lock, Duration::from_millis(100))
                .unwrap()
                .0;
            //effective_lock = self.condvar.wait(effective_lock).unwrap();
            //TODO: rather pass the error to the waiter
            if *self.is_poisoned.read().unwrap() == true {
                panic!()
            }
        }
    }

    fn get_is_poisoned(&self) -> bool {
        *self.is_poisoned.read().unwrap()
    }

    fn poison(&self) {
        let mut is_poisoned = self.is_poisoned.write().unwrap();
        *is_poisoned = true;
    }

    fn add_error(&self, err: RecurseError, keep_going: bool) {
        let mut errors_lock = self.errors.lock().unwrap();
        if !keep_going {
            self.poison();
        };
        errors_lock.push(err);
    }
}

fn kodi_recurse_inner_thread<
    'a,
    T: 'static + Clone + Send,
    F: 'static + Clone + Fn(&mut RecurseInfo, T) -> T + Clone + Send,
    C: 'static + Fn(&mut RecurseInfo, &T) -> bool + Clone + Send,
>(
    kodi: Arc<Kodi>,
    func: F,
    skip_this_and_children: C,
    parent: Option<Page>,
    access: PathAccessData,
    data: T,
    spawn_thread_data: Arc<SpawnNewThreadData>,
    have_decremented: Arc<AtomicBool>,
    keep_going: bool
) {
    //parent, access, data, Fn(Option<Page>, PathAccessData, T)
    let mut actual_page = match kodi.invoke_sandbox(&access) {
        Ok(result) => match result {
            KodiResult::Content(p) => p,
            other => panic!("can't use {:?} in a recursive context", other),//TODO: remove this panic
        },
        Err(err) => {
            spawn_thread_data.add_error(RecurseError::KodiCallError(access.clone(), Arc::new(err)), keep_going);
            spawn_thread_data.decrement_worker();
            return
        },
    };

    let mut sub_content_from_parent = None;

    if let Some(resolved_listitem) = actual_page.resolved_listitem.as_mut() {
        if let Some(parent_page) = parent.as_ref() {
            for parent_sub_content in &parent_page.sub_content {
                sub_content_from_parent = Some(parent_sub_content);
                if parent_sub_content.url == access.path {
                    resolved_listitem.extend(parent_sub_content.listitem.clone());
                };
            }
        };
    };

    let mut info = RecurseInfo {
        page: &actual_page,
        sub_content_from_parent,
        access: &access,
        errors: Vec::new(),
    };

    let skip_this_element = skip_this_and_children(&mut info, &data);

    for error in info.errors.drain(..) {
        spawn_thread_data.add_error(error, keep_going);
    };

    if skip_this_element {
        spawn_thread_data.decrement_worker();
        have_decremented.store(true, Ordering::Relaxed);
        return;
    }


    let data_for_child = func(&mut info, data);

    for error in info.errors.drain(..) {
        spawn_thread_data.add_error(error, keep_going);
    };

    spawn_thread_data.decrement_worker();
    have_decremented.store(true, Ordering::Relaxed);

    //TODO: do not spawn more than enought active thread
    let mut threads: Vec<(JoinHandle<_>, Arc<AtomicBool>, PathAccessData)> = Vec::new();

    for sub_content in &actual_page.sub_content {
        let last_one_possible = spawn_thread_data.wait_to_spawn_child_then_increment_worker();

        let parent_page = Some(actual_page.clone());
        let child_data_cloned = data_for_child.clone();
        let child_access =
            PathAccessData::new(sub_content.url.clone(), None, access.config.clone());
        let child_access_log = child_access.clone();
        let kodi_cloned = kodi.clone();
        let func_cloned = func.clone();
        let skip_this_and_children = skip_this_and_children.clone();
        let spawn_thread_data_cloned = spawn_thread_data.clone();
        let child_have_decrement = Arc::new(AtomicBool::new(false));
        let child_have_decrement_cloned = child_have_decrement.clone();
        let keep_going_cloned = keep_going;

        let handle = thread::spawn(move || {
            kodi_recurse_inner_thread(
                kodi_cloned,
                func_cloned,
                skip_this_and_children,
                parent_page,
                child_access,
                child_data_cloned,
                spawn_thread_data_cloned,
                child_have_decrement_cloned,
                keep_going_cloned
            )
        });

        // ensure at least one thread is working
        if last_one_possible {
            if let Err(_err) = handle.join() {
                if child_have_decrement.fetch_or(false, Ordering::Relaxed) == false {
                    spawn_thread_data.decrement_worker();
                };
                spawn_thread_data.add_error(RecurseError::ThreadPanicked(child_access_log), keep_going);
            }
        } else {
            threads.push((handle, child_have_decrement, child_access_log));
        }

        if spawn_thread_data.get_is_poisoned() {
            break
        }
    }

    //TODO: dedup

    for thread in threads.drain(..) {
        if let Err(err) = thread.0.join() {
            if thread.1.fetch_or(false, Ordering::Relaxed) == false {
                spawn_thread_data.decrement_worker();
            };
            spawn_thread_data.poison();
            println!(
                "a thread panicked while evaluating the access {:?}. cleanly exiting. (err: {:?})",
                thread.2, err
            );
        }
    }
}

//TODO: single thread implementation
pub fn kodi_recurse_par<
    'a,
    T: 'static + Clone + Send,
    F: 'static + Fn(&mut RecurseInfo, T) -> T + Clone + Send,
    C: 'static + Fn(&mut RecurseInfo, &T) -> bool + Clone + Send,
>(
    kodi: Kodi,
    access: PathAccessData,
    data: T,
    func: F,
    skip_this_and_children: C,
    thread_nb: usize,
    keep_going: bool,
) -> Vec<RecurseError> {
    if thread_nb == 0 {
        panic!()
    }

    let kodi = Arc::new(kodi);

    let spawn_thread_data = Arc::new(SpawnNewThreadData {
        thread_nb,
        effective_task: Arc::new(Mutex::new(1)),
        condvar: Condvar::new(),
        is_poisoned: RwLock::new(false),
        errors: Mutex::new(Vec::new()),
    });

    let spawn_thread_data_cloned = spawn_thread_data.clone();
    let original_thread = thread::spawn(move || {
        kodi_recurse_inner_thread(
            kodi,
            func,
            skip_this_and_children,
            None,
            access,
            data,
            spawn_thread_data_cloned,
            Arc::new(AtomicBool::new(false)),
            keep_going,
        )
    });

    original_thread.join().unwrap();

    return spawn_thread_data.errors.lock().unwrap().clone();
}
