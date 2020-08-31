use crate::data::{KodiResult, Page};
use crate::{Kodi, PathAccessData};

use std::time::Duration;
use std::sync::{Arc, Mutex, Condvar, RwLock, atomic::AtomicBool, atomic::Ordering};
use std::thread;
use std::thread::JoinHandle;

struct SpawnNewThreadData {
    thread_nb: usize,
    effective_task: Arc<Mutex<usize>>,
    condvar: Condvar,
    is_poisoned: RwLock<bool>,
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
                return *effective_lock == self.thread_nb
            };
            effective_lock = self.condvar.wait_timeout(effective_lock, Duration::from_millis(100)).unwrap().0;
            //effective_lock = self.condvar.wait(effective_lock).unwrap();
            if *self.is_poisoned.read().unwrap() == true {
                panic!()
            }
        }
    }

    fn poison(&self) {
        let mut is_poisoned = self.is_poisoned.write().unwrap();
        *is_poisoned = true;
    }
}

fn kodi_recurse_inner_thread<
    'a,
    T: 'static + Clone + Send,
    F: 'static + Clone + Fn(&Page, Option<T>) -> Option<T> + Clone + Send,
    C: 'static + Fn(&Page, Option<&T>) -> bool + Clone + Send,
>(
    kodi: Arc<Kodi>,
    func: F,
    skip_this_and_children: C,
    parent: Option<Page>,
    access: PathAccessData,
    data: Option<T>,
    spawn_thread_data: Arc<SpawnNewThreadData>,
    have_decremented: Arc<AtomicBool>,
) {
    //parent, access, data, Fn(Option<Page>, PathAccessData, Option<T>)
    let mut actual_page = match kodi.invoke_sandbox(&access).unwrap() {
        KodiResult::Content(p) => p,
        other => panic!("can't use {:?} in a recursive context", other),
    };

    if let Some(resolved_listitem) = actual_page.resolved_listitem.as_mut() {
        if let Some(parent_page) = parent.as_ref() {
            for parent_sub_content in &parent_page.sub_content {
                if parent_sub_content.url == access.path {
                    resolved_listitem.extend(parent_sub_content.listitem.clone());
                };
            }
        };
    };

    let skip_this_element = skip_this_and_children(&actual_page, data.as_ref());

    if skip_this_element {
        spawn_thread_data.decrement_worker();
        have_decremented.store(true, Ordering::Relaxed);
        return;
    }

    let data_for_child = func(&actual_page, data);

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

        let handle = thread::spawn(move || {kodi_recurse_inner_thread(kodi_cloned, func_cloned, skip_this_and_children, parent_page, child_access, child_data_cloned, spawn_thread_data_cloned, child_have_decrement_cloned)});

        // ensure at least one thread is working
        if last_one_possible {
            if let Err(err) = handle.join() {
                if child_have_decrement.fetch_or(false, Ordering::Relaxed) == false {
                    spawn_thread_data.decrement_worker();
                };
                spawn_thread_data.poison();
                for thread in threads.drain(..) {
                    if let Err(_) = thread.0.join() {
                        if thread.1.fetch_or(false, Ordering::Relaxed) == false {
                            spawn_thread_data.decrement_worker();
                        };
                    }
                }
                panic!("a thread panicked while evaluating the access {:?}. cleanly exiting. (err: {:?})", child_access_log, err);
            }
        } else {
            threads.push((handle, child_have_decrement, child_access_log));
        }
    }

    //TODO: dedup

    for thread in threads.drain(..) {
        if let Err(err) = thread.0.join() {
            if thread.1.fetch_or(false, Ordering::Relaxed) == false {
                spawn_thread_data.decrement_worker();
            };
            spawn_thread_data.poison();
            println!("a thread panicked while evaluating the access {:?}. cleanly exiting. (err: {:?})", thread.2, err);
        }
    }
}

//TODO: single thread implementation
pub fn kodi_recurse_par<
    'a,
    T: 'static + Clone + Send,
    F: 'static + Fn(&Page, Option<T>) -> Option<T> + Clone + Send,
    C: 'static + Fn(&Page, Option<&T>) -> bool + Clone + Send,
>(
    kodi: Kodi,
    access: PathAccessData,
    data: Option<T>,
    func: F,
    skip_this_and_children: C,
    thread_nb: usize,
) {
    if thread_nb == 0 {
        panic!()
    }

    let kodi = Arc::new(kodi);

    let spawn_thread_data = Arc::new(SpawnNewThreadData {
        thread_nb,
        effective_task: Arc::new(Mutex::new(1)),
        condvar: Condvar::new(),
        is_poisoned: RwLock::new(false),
    });
    let original_thread = thread::spawn(move || kodi_recurse_inner_thread(kodi, func, skip_this_and_children, None, access, data, spawn_thread_data, Arc::new(AtomicBool::new(false))));

    original_thread.join().unwrap();
}
