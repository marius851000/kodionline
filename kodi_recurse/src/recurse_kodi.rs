use crate::report::RecurseReport;
use crate::AppArgument;
use crate::RecurseOption;
use crate::ReportBuilder;

use indicatif::ProgressBar;
use kodi_rust::data::{KodiResult, Page, SubContent};
use kodi_rust::{Kodi, PathAccessData};

use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use log::error;

#[derive(Clone)]
pub struct RecurseInfo<'a> {
    page: &'a Page,
    pub sub_content_from_parent: Option<&'a SubContent>,
    pub access: &'a PathAccessData,
    pub parent_access: Option<&'a PathAccessData>,
    pub errors: Vec<RecurseReport>,
}

impl<'a> RecurseInfo<'a> {
    pub fn get_page(&self) -> &'a Page {
        return self.page;
    }

    pub fn get_sub_content_from_parent(&self) -> Option<&'a SubContent> {
        return self.sub_content_from_parent;
    }

    pub fn get_access(&self) -> &'a PathAccessData {
        self.access
    }

    pub fn add_error_string(&mut self, error_message: String) {
        self.errors.push(RecurseReport::CalledReport(
            self.access.clone(),
            self.parent_access.map(|x| x.clone()),
            ReportBuilder::new_error(error_message),
        ));
    }

    pub fn add_report(&mut self, report: ReportBuilder) {
        self.errors.push(RecurseReport::CalledReport(
            self.access.clone(),
            self.parent_access.map(|x| x.clone()),
            report,
        ));
    }
}

//TODO: put keep going here
struct SpawnNewThreadData {
    thread_nb: usize,
    effective_task: Arc<Mutex<usize>>,
    condvar: Condvar,
    is_poisoned: RwLock<bool>,
    errors: Mutex<Vec<RecurseReport>>, //TODO: custom type for more display configuration
    progress_bar: Option<ProgressBar>,
    app_argument: AppArgument,
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

    fn add_error(&self, err: RecurseReport, keep_going: bool) {
        self.progress_bar.as_ref().map(|bar| {
            bar.println(err.get_text_to_print(&self.app_argument));
            bar.println("\n");
        });
        let mut errors_lock = self.errors.lock().unwrap();
        if !keep_going {
            self.poison();
        };
        errors_lock.push(err);
    }

    fn increment_finished_task(&self) {
        self.progress_bar.as_ref().map(|bar| bar.inc(1));
    }

    fn add_task(&self, to_add: u64) {
        self.progress_bar.as_ref().map(|bar| bar.inc_length(to_add));
    }

    fn finish(&self) {
        self.progress_bar.as_ref().map(|bar| bar.finish());
    }
}

fn kodi_recurse_inner_thread<
    'a,
    T: 'static + Clone + Send,
    F: 'static + Clone + Fn(&mut RecurseInfo, T) -> Option<T> + Clone + Send,
    C: 'static + Fn(&mut RecurseInfo, &T) -> bool + Clone + Send,
>(
    kodi: Arc<Kodi>,
    func: F,
    skip_this_and_children: C,
    parent: Option<(Page, PathAccessData)>,
    access: PathAccessData,
    data: T,
    spawn_thread_data: Arc<SpawnNewThreadData>,
    have_decremented: Arc<AtomicBool>,
    keep_going: bool,
    sub_content_from_parent: Option<SubContent>,
) {
    let (parent_page, parent_access) = match parent {
        Some((p, a)) => (Some(p), Some(a)),
        None => (None, None),
    };

    //parent, access, data, Fn(Option<Page>, PathAccessData, T)
    let actual_page = match kodi.invoke_sandbox(&access) {
        Ok(result) => match result {
            KodiResult::Content(p) => p,
            other => panic!("can't use {:?} in a recursive context", other), //TODO: remove this panic
        },
        Err(err) => {
            spawn_thread_data.add_error(
                RecurseReport::KodiCallError(access.clone(), Arc::new(err)),
                keep_going,
            );
            spawn_thread_data.increment_finished_task();
            spawn_thread_data.decrement_worker();
            return;
        }
    };

    let mut info = RecurseInfo {
        page: &actual_page,
        sub_content_from_parent: sub_content_from_parent.as_ref(),
        access: &access,
        parent_access: parent_access.as_ref(),
        errors: Vec::new(),
    };

    let skip_this_element = skip_this_and_children(&mut info, &data);

    for error in info.errors.drain(..) {
        spawn_thread_data.add_error(error, keep_going);
    }

    if skip_this_element {
        spawn_thread_data.decrement_worker();
        spawn_thread_data.increment_finished_task();
        have_decremented.store(true, Ordering::Relaxed);
        return;
    }

    let func_result = func(&mut info, data);

    for error in info.errors.drain(..) {
        spawn_thread_data.add_error(error, keep_going);
    }

    // the 3 following line shouldn't crash
    spawn_thread_data.decrement_worker();
    spawn_thread_data.increment_finished_task();
    have_decremented.store(true, Ordering::Relaxed);

    let data_for_child = match func_result {
        Some(v) => v,
        None => return,
    };
    //TODO: do not spawn more than enought active thread
    let mut threads: Vec<(JoinHandle<_>, Arc<AtomicBool>, PathAccessData)> = Vec::new();

    spawn_thread_data.add_task(actual_page.sub_content.len() as u64);

    for sub_content in &actual_page.sub_content {
        let last_one_possible = spawn_thread_data.wait_to_spawn_child_then_increment_worker();

        let parent_page = actual_page.clone();
        let parent_access = access.clone();
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
        let sub_content_from_parent_cloned = sub_content.clone();

        let handle = thread::spawn(move || {
            kodi_recurse_inner_thread(
                kodi_cloned,
                func_cloned,
                skip_this_and_children,
                Some((parent_page, parent_access)),
                child_access,
                child_data_cloned,
                spawn_thread_data_cloned,
                child_have_decrement_cloned,
                keep_going_cloned,
                Some(sub_content_from_parent_cloned),
            )
        });

        // ensure at least one thread is working
        if last_one_possible {
            if let Err(_err) = handle.join() {
                if child_have_decrement.fetch_or(false, Ordering::Relaxed) == false {
                    spawn_thread_data.decrement_worker();
                    spawn_thread_data.increment_finished_task();
                };
                spawn_thread_data.add_error(
                    RecurseReport::ThreadPanicked(child_access_log, Some(access.clone())),
                    keep_going,
                );
            }
        } else {
            threads.push((handle, child_have_decrement, child_access_log));
        }

        if spawn_thread_data.get_is_poisoned() {
            break;
        }
    }

    for thread in threads.drain(..) {
        if let Err(_) = thread.0.join() {
            if thread.1.fetch_or(false, Ordering::Relaxed) == false {
                spawn_thread_data.decrement_worker();
                spawn_thread_data.increment_finished_task();
            };
            spawn_thread_data.add_error(
                RecurseReport::ThreadPanicked(thread.2, Some(access.clone())),
                keep_going,
            );
        }
    }
}

//TODO: single thread implementation
pub fn kodi_recurse_par<
    'a,
    T: 'static + Clone + Send,
    F: 'static + Fn(&mut RecurseInfo, T) -> Option<T> + Clone + Send,
    C: 'static + Fn(&mut RecurseInfo, &T) -> bool + Clone + Send,
>(
    mut option: RecurseOption,
    data: T,
    func: F,
    skip_this_and_children: C,
) -> Vec<RecurseReport> {
    if option.thread_nb == 0 {
        error!("kodi_recurse_par require to have at least one active thread ! Making use of 1 thread instead of 0.");
        option.thread_nb = 1;
    }

    let kodi = Arc::new(option.kodi);

    let spawn_thread_data = Arc::new(SpawnNewThreadData {
        thread_nb: option.thread_nb,
        effective_task: Arc::new(Mutex::new(1)),
        condvar: Condvar::new(),
        is_poisoned: RwLock::new(false),
        errors: Mutex::new(Vec::new()),
        progress_bar: option.progress_bar,
        app_argument: option.app_argument,
    });

    let parent_data = match option.top_parent {
        Some(parent_access) => Some((
            match kodi.invoke_sandbox(&parent_access) {
                Ok(r) => match r {
                    KodiResult::Content(c) => c,
                    _ => todo!(), //TODO: RecurseReport value for this
                },
                Err(e) => {
                    spawn_thread_data.finish();
                    return vec![RecurseReport::KodiCallError(parent_access, Arc::new(e))];
                }
            },
            parent_access,
        )),
        None => None,
    };

    let spawn_thread_data_cloned = spawn_thread_data.clone();
    let access_cloned = option.top_access.clone();
    let keep_going_cloned = option.keep_going.clone();
    let original_thread = thread::spawn(move || {
        kodi_recurse_inner_thread(
            kodi,
            func,
            skip_this_and_children,
            parent_data,
            access_cloned,
            data,
            spawn_thread_data_cloned,
            Arc::new(AtomicBool::new(false)),
            keep_going_cloned,
            None,
        )
    });

    match original_thread.join() {
        Ok(_) => (),
        Err(_) => spawn_thread_data.add_error(
            RecurseReport::ThreadPanicked(option.top_access, None),
            option.keep_going,
        ),
    }

    spawn_thread_data.finish();
    return spawn_thread_data.errors.lock().unwrap().clone();
}
