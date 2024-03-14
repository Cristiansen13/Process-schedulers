use crate::scheduler::{
    Process, ProcessState, Pid, Scheduler, SchedulingDecision, StopReason, Syscall, SyscallResult,
};
use std::{num::NonZeroUsize, collections::VecDeque};

pub struct RoundRobinProcess {
    pid: Pid,
    state: ProcessState,
    priority: i8,
    timings: (usize, usize, usize),
    remaining: usize,
    sleep_time: usize,
    total_time: usize,
}

impl RoundRobinProcess {
    pub fn new(pid: Pid, state: ProcessState, priority: i8, timings: (usize, usize, usize), remaining: usize) -> Self {
        RoundRobinProcess {
            pid,
            state,
            priority,
            timings,
            remaining,
            sleep_time: 0,
            total_time: remaining,
        }
    }
    pub fn set_state(&mut self, new_state: ProcessState) {
        self.state = new_state;
    }
}

impl Process for RoundRobinProcess {
    fn pid(&self) -> Pid {
        self.pid
    }

    fn state(&self) -> ProcessState {
        self.state
    }
    

    fn timings(&self) -> (usize, usize, usize) {
        self.timings
    }

    fn priority(&self) -> i8 {
        self.priority
    }

    fn extra(&self) -> String {
        format!("")
    }
}

pub struct RoundRobinScheduler {
    processes: Vec<RoundRobinProcess>,
    ready_queue: VecDeque<Pid>,
    sleep_queue: VecDeque<Pid>,
    timeslice: NonZeroUsize,
    minimum_remaining_timeslice: usize,
    nr_processes: usize,
    time: usize,
}

impl RoundRobinScheduler {
    pub fn new(timeslice: NonZeroUsize, minimum_remaining_timeslice: usize) -> Self {
        Self {
            processes: Vec::new(),
            ready_queue: VecDeque::new(),
            sleep_queue: VecDeque::new(),
            timeslice,
            minimum_remaining_timeslice,
            nr_processes: 0,
            time: 0,
        }
    }
}



impl Scheduler for RoundRobinScheduler {
    fn next(&mut self) -> SchedulingDecision {
        if self.ready_queue.len() > 0 {
            let mut i = 0;
            for pid in self.ready_queue.iter() {
                if *pid == 1 {
                    i += 1;
                }
            }
            for pid in self.sleep_queue.iter() {
                if *pid == 1 {
                    i += 1;
                }
            }
            if i == 0 {
                for process in self.processes.iter_mut() {
                    process.set_state(ProcessState::Ready);
                }
                return SchedulingDecision::Panic;
            }
        }
        if let Some(pid) = self.ready_queue.pop_front() {
            self.ready_queue.push_front(pid);
            let process_index = self
                .processes
                .iter()
                .position(|p| p.pid() == pid)
                .expect("Process not found in the list");
            if self.processes[process_index].sleep_time > 0 {
                self.processes[process_index].remaining = self.processes[process_index].total_time;
                self.processes[process_index].timings.0 += self.processes[process_index].sleep_time;
                self.processes[process_index].sleep_time = 0;
            }
            if let Some(remaining) = NonZeroUsize::new(self.processes[process_index].remaining) {
                if remaining.get() >= self.minimum_remaining_timeslice{
                    self.processes[process_index].set_state(ProcessState::Running);
                    return SchedulingDecision::Run { pid:pid, timeslice: remaining };
                } else {
                    self.ready_queue.push_back(pid);
                    self.processes[process_index].set_state(ProcessState::Ready);
                }
            } else {
                self.ready_queue.push_back(pid);
                self.processes[process_index].set_state(ProcessState::Ready);
            }
            if let Some(pid) = self.ready_queue.pop_front() {
                self.ready_queue.push_front(pid);
                let process_index = self
                .processes
                .iter()
                .position(|p| p.pid() == pid)
                .expect("Process not found in the list");
                if let Some(remaining) = NonZeroUsize::new(self.processes[process_index].remaining) {
                    self.processes[process_index].set_state(ProcessState::Running);
                    return SchedulingDecision::Run { pid:pid, timeslice: remaining };
                } else {
                    self.processes[process_index].set_state(ProcessState::Running);
                    return SchedulingDecision::Run { pid:pid, timeslice: self.timeslice };
                }
            } else {
                SchedulingDecision::Done
            }    
        } else if let Some(pid) = self.sleep_queue.pop_front() { 
            let process_index = self
                .processes
                .iter()
                .position(|p| p.pid() == pid)
                .expect("Process not found in the list");
            let sleep = NonZeroUsize::new(self.processes[process_index].sleep_time).unwrap();
            self.ready_queue.push_back(pid);
            SchedulingDecision::Sleep(sleep)
        }else{
            SchedulingDecision::Done
        }
    }
    
    fn stop(&mut self, reason: StopReason) -> SyscallResult {
        match reason {
            StopReason::Syscall { syscall, remaining } => {
                match syscall {
                    Syscall::Fork(process_priority) => {
                        if let Some(pid) = self.ready_queue.pop_front() {
                            self.ready_queue.push_front(pid);
                            let process_index = self
                                .processes
                                .iter()
                                .position(|p| p.pid() == pid)
                                .expect("Process not found in the list");
                            self.time += self.processes[process_index].remaining - remaining;
                            self.processes[process_index].timings.0 += self.processes[process_index].remaining - remaining;
                            self.processes[process_index].timings.1 += 1;
                            self.processes[process_index].timings.2 += self.processes[process_index].remaining - remaining - 1;
                            for i in 1..self.ready_queue.len() {
                                let pid = self.ready_queue.get(i).unwrap();
                                let process_index = self
                                    .processes
                                    .iter()
                                    .position(|p| p.pid() == *pid)
                                    .expect("Process not found in the list");
                                self.processes[process_index].timings.0 += self.processes[process_index].remaining - remaining;
                            }
                            self.processes[process_index].remaining = remaining;
                        }
                        let new_pid = Pid::new((self.nr_processes + 1).try_into().unwrap());
                        self.nr_processes += 1;
                        let new_process = RoundRobinProcess::new(
                            new_pid,
                            ProcessState::Ready,
                            process_priority,
                            (0, 0, 0),
                            self.timeslice.into(),
                        );
                        self.processes.push(new_process);
                        self.ready_queue.push_back(new_pid);
                        return SyscallResult::Pid(new_pid);
                    }
                    Syscall::Sleep(amount_of_time) => {
                        if let Some(pid) = self.ready_queue.pop_front() {
                            self.ready_queue.push_front(pid);
                            let process_index = self
                                .processes
                                .iter()
                                .position(|p| p.pid() == pid)
                                .expect("Process not found in the list");
                            self.time += self.processes[process_index].remaining - remaining;
                            self.processes[process_index].timings.0 += self.processes[process_index].remaining - remaining;
                            self.processes[process_index].timings.1 += 1;
                            self.processes[process_index].timings.2 += self.processes[process_index].remaining - remaining - 1;
                            for i in 1..self.ready_queue.len() {
                                let pid = self.ready_queue.get(i).unwrap();
                                let process_index = self
                                    .processes
                                    .iter()
                                    .position(|p| p.pid() == *pid)
                                    .expect("Process not found in the list");
                                self.processes[process_index].timings.0 += self.processes[process_index].remaining - remaining;
                            }
                            self.processes[process_index].remaining = remaining;
                        }
                        if let Some(pid) = self.ready_queue.pop_front() {
                            let process_index = self
                                .processes
                                .iter()
                                .position(|p| p.pid() == pid)
                                .expect("Process not found in the list");
                            self.processes[process_index].sleep_time = amount_of_time;
                            let event = None;
                            self.processes[process_index].set_state(ProcessState::Waiting {event});
                            self.sleep_queue.push_back(pid);
                        }
                        
                        return SyscallResult::Success;
                    }
                    Syscall::Wait(_event_number) => {
                        return SyscallResult::Success;
                    }
                    Syscall::Signal(_event_number) => {
                        return SyscallResult::Success;
                    }
                    Syscall::Exit => {
                        if let Some(pid) = self.ready_queue.pop_front() {
                            let process_index = self
                                .processes
                                .iter()
                                .position(|p| p.pid() == pid)
                                .expect("Process not found in the list");
                            for i in 0..self.ready_queue.len() {
                                let new_pid = self.ready_queue.get(i).unwrap();
                                let new_process_index = self
                                    .processes
                                    .iter()
                                    .position(|p| p.pid() == *new_pid)
                                    .expect("Process not found in the list");
                                self.processes[new_process_index].timings.0 += self.processes[process_index].remaining - remaining;
                            }
                            self.time += self.processes[process_index].remaining - remaining;
                            self.processes.retain(|p| p.pid() != pid);
                        }
                        return SyscallResult::Success;
                    }    
                }
            }
            StopReason::Expired => {
                if let Some(pid) = self.ready_queue.pop_front() {
                    self.processes
                        .iter_mut()
                        .find(|p| p.pid() == pid)
                        .unwrap() 
                        .set_state(ProcessState::Ready);
                    let process_index = self
                        .processes
                        .iter()
                        .position(|p| p.pid() == pid)
                        .expect("Process not found in the list");
                    self.time += self.processes[process_index].remaining;
                    self.processes[process_index].timings.2 += self.processes[process_index].remaining;
                    self.processes[process_index].timings.0 += self.processes[process_index].remaining;
                    
                    for i in 0..self.ready_queue.len() {
                        let pid = self.ready_queue.get(i).unwrap();
                        let new_process_index = self
                            .processes
                            .iter()
                            .position(|p| p.pid() == *pid)
                            .expect("Process not found in the list");
                        self.processes[new_process_index].timings.0 += self.processes[process_index].remaining;
                    }
                    self.processes[process_index].remaining = self.timeslice.into();
                    self.ready_queue.push_back(pid);
                }
            }
        }
        SyscallResult::Success
    }

    fn list(&mut self) -> Vec<&dyn Process> {
        self.processes.iter().map(|p| p as &dyn Process).collect::<Vec<&dyn Process>>()
    }
}
