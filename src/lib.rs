use duct::{cmd, Expression, ReaderHandle};
use std::ffi::OsString;
use std::fmt::Error;
use std::io::{BufRead, BufReader};

use std::sync::Arc;
use std::thread::JoinHandle;
use std::{io, thread};

/// Various events associated with process's life-cycle
///
#[derive(Debug)]
pub enum ProcessEvent {
    /// Default value placeholder
    _Unknown,
    /// Process is starting but not yet started!
    Starting,
    /// Process is started
    Started,
    /// Error occurred while starting the process itself
    StartError,
    /// Process started but error occurred during reading the output data
    IOError,
    /// Process started and output data reader reached to the EOF, means process's output data is unavailable
    IOEof,
    /// Process started and a line from the output data is available now
    IOData,
    /// Process started and during IOData reading based on the API consumer's decision the callback returned [`Some(false)`] ,
    /// which means process's exit request is submitted
    ExitRequested,
    /// Kill API was used to kill the process
    KillRequested,
    /// Process which was started earlier now exited
    Exited,
    /// A error occurred while killing/stopping the process
    KillError,
}

/// Various fields related to the process
///
pub struct ProcessData<'a> {
    /// Request data
    request: Option<Arc<ProcessRequest>>,
    /// Line number from output of the Process's STDOUT & STDERR
    pub line_number: i64,
    /// A single line data from output of the Process's STDOUT & STDERR
    pub line: String,
    /// Internal reader handle for managing the process
    reader: Option<&'a ReaderHandle>,
}

impl ProcessData<'_> {
    /// Create a new instance of the [`ProcessData`]
    ///
    pub fn new() -> Self {
        Self {
            request: None,
            line_number: 0,
            line: String::new(),
            reader: None,
        }
    }
    /// Kill the running process
    pub fn kill(&self) -> io::Result<()> {
        Ok(if self.reader.is_some() {
            check_and_trigger_callback(
                &self.request.as_ref().unwrap(),
                &ProcessEvent::KillRequested,
                &self,
            );
            return self.reader.as_ref().unwrap().kill();
        })
    }

    /// Get the list of child pids
    pub fn child_pids(&self) -> Vec<u32> {
        if self.reader.is_some() {
            return self.reader.as_ref().unwrap().pids();
        }
        return vec![];
    }
}

/// check if the callback is registered and if yes then trigger it wi the supplied data
fn check_and_trigger_callback(
    request: &Arc<ProcessRequest>,
    event: &ProcessEvent,
    data: &ProcessData,
) -> Option<bool> {
    if request.callback.as_ref().is_some() {
        return request.callback.as_ref().unwrap()(event, data);
    };
    Some(true)
}

unsafe impl Sync for ProcessRequest {}
unsafe impl Send for ProcessRequest {}

/// A request structure to start a process
pub struct ProcessRequest {
    /// Custom unique numeric id to relate the various callbacks for a particular process execution session
    pub request_id: u32,
    /// Use shell mode or direct executable path based execution
    pub use_shell: bool,
    /// Use blocking or non blocking mode using internal threads
    pub non_blocking_mode: bool,
    /// (2D Array) Vector of command line along with arguments. For a single command line one vector element is enough. For the pipe line use case where output of one command to provide to the next command, use Vector of command lines.
    pub cmd_line: Vec<Vec<String>>,
    /// Register callback to get various events and process output, for no callbacks use None
    pub callback: Option<Arc<dyn Fn(&ProcessEvent, &ProcessData) -> Option<bool> + 'static>>,
}

impl ProcessRequest {
    /**
     Run a process based on the provided process request which is events based multi process execution(blocking & non-blocking modes) in parallel and with data streaming
     Generates various events [`ProcessEvent`] according to the process's life-cycle, process's information and data [`ProcessData`] associated with that event
     # Arguments
     ProcessRequest : [`ProcessRequest`] // A request structure to start a process
     # Return
     For Blocking mode it returns [`None`] and for Non-Blocking mode it will return [`Some(io::Result<JoinHandle<()>>)`], so the caller can join & wait for the process completion if needed!
     # Examples
     ```
    // Setup callback for the process events and data streaming
    //
    // use process_events_streaming::{ProcessRequest, ProcessData, ProcessEvent};
    // use std::{thread, time::Duration};
    //  let callback = |status: &ProcessEvent, data: &ProcessData| -> Option<bool> {
    //         match status {
    //             ProcessEvent::Started => {
    //                 println!(
    //                     "Event {:?} | req-id {}  | Pids: {:?}",
    //                     status,
    //                     data.request.as_ref().unwrap().request_id,
    //                     data.pids
    //                 );
    //             }
    //             ProcessEvent::IOData => {
    //                 println!(
    //                     "Event {:?} | req-id {} | # {} : {}",
    //                     status,
    //                     data.request.as_ref().unwrap().request_id,
    //                     data.line_number,
    //                     data.line
    //                 );
    //
    //                 //demo how to kill/stop
    //                 // //using kill api
    //                 //_ = data.kill();
    //                 // //or return false to exit the process, based on the line_number value
    //                 // if data.line_number == 1 {
    //                 //     return Some(false);
    //                 // }
    //                 // // or return false to exit the process, if a condition is true based on the output data
    //                 // if data.line.contains("Sandy") {
    //                 //     return Some(false);
    //                 // }
    //             }
    //             other => {
    //                 if !data.line.is_empty() {
    //                     println!(
    //                         "Event {:?} | req-id {} | additional detail(s): {}",
    //                         other,
    //                         data.request.as_ref().unwrap().request_id,
    //                         data.line
    //                     );
    //                 } else {
    //                     println!(
    //                         "Event {:?} | req-id {}",
    //                         other,
    //                         data.request.as_ref().unwrap().request_id
    //                     );
    //                 }
    //             }
    //         }
    //         Some(true)
    //     };
    //
    //
    //    ProcessRequest::start(ProcessRequest {
    //         request_id: 161,
    //         callback: Some(Arc::new(callback)),
    //         use_shell: true,
    //         cmd_line: vec![vec![String::from("dir")], vec![String::from("sort")]],
    //         non_blocking_mode: true,
    //     });
    //
    //     //check & wait for the non blocking mode
    //     if result1.is_some() {
    //         if result1.as_ref().unwrap().is_ok() {
    //             println!(
    //                 "Start - join waiting over in non blocking mode {:?}",
    //                 result1
    //                     .unwrap()
    //                     .unwrap()
    //                     .join()
    //                     .expect("Couldn't join on the associated thread")
    //             );
    //         } else {
    //             println!(
    //                 "Start - Error in non blocking mode {:?}",
    //                 result1.unwrap().err()
    //             );
    //         }
    //     } else {
    //         println!("Start - It was a blocking mode, so nothing to wait for!");
    //     }
    //
    //    println!(
    //     "test_using_sh_output_streaming, start calc in windows {:?}",
    //     ProcessRequest::start(ProcessRequest {
    //         request_id: 191,
    //         callback: Some(Arc::new(callback)),
    //         use_shell: true,
    //         cmd_line: vec![vec![String::from("calc")]],
    //         non_blocking_mode: false,
    //     }));
     ```
    */
    pub fn start(process_request: ProcessRequest) -> Option<io::Result<JoinHandle<()>>> {
        let request = Arc::new(process_request);
        if request.non_blocking_mode {
            let thread_handle = thread::Builder::new()
                .name(format!("pes_th_rq_{}", request.request_id).into())
                .spawn(move || {
                    start_process(request);
                });
            return Some(thread_handle);
        } else {
            start_process(request);
        }
        None
    }
}

fn start_process(request: Arc<ProcessRequest>) {
    let mut process_data = ProcessData::new();
    process_data.line.clear();
    process_data.request = Some(Arc::clone(&request));
    if request.as_ref().cmd_line.len() == 0 || request.as_ref().cmd_line[0].len() == 0 {
        process_data
            .line
            .push_str(format!("{:?}", "Command line - arguments are unavailable!").as_str());
        check_and_trigger_callback(&request, &ProcessEvent::StartError, &process_data);
        return;
    }
    process_data.line.push_str(
        format!(
            "Executing in thread-context -> id: {:?}, name: {:?}",
            thread::current().id(),
            thread::current().name()
        )
        .as_str(),
    );
    check_and_trigger_callback(&request, &ProcessEvent::Starting, &process_data);

    let process_req = &request;
    let stdout_reader = handle_pipeline(&request).stderr_to_stdout().reader();
    process_data.reader = Some(stdout_reader.as_ref().unwrap());
    match stdout_reader.as_ref() {
        Ok(stdout_reader) => {
            check_and_trigger_callback(process_req, &ProcessEvent::Started, &process_data);
            let mut buffer_reader = BufReader::new(stdout_reader);
            loop {
                process_data.line.clear();
                let result = buffer_reader.read_line(&mut process_data.line);
                match result {
                    Ok(result) if result == 0 => {
                        check_and_trigger_callback(
                            process_req,
                            &ProcessEvent::IOEof,
                            &process_data,
                        );
                        break;
                    }
                    Ok(_result) => {
                        process_data.line_number += 1;
                        match check_and_trigger_callback(
                            process_req,
                            &ProcessEvent::IOData,
                            &process_data,
                        ) {
                            Some(value) if value == false => {
                                check_and_trigger_callback(
                                    process_req,
                                    &ProcessEvent::ExitRequested,
                                    &process_data,
                                );
                                break;
                            }
                            _other => {}
                        }
                    }
                    Err(error) => {
                        process_data.line.push_str(format!("{:?}", error).as_str());
                        check_and_trigger_callback(
                            process_req,
                            &ProcessEvent::IOError,
                            &process_data,
                        );
                        break;
                    }
                }
            }
            process_data.line.clear();
            let exit_result = stdout_reader.kill();

            match exit_result {
                Ok(_) => {
                    check_and_trigger_callback(process_req, &ProcessEvent::Exited, &process_data);
                }
                Err(_) => {
                    check_and_trigger_callback(
                        process_req,
                        &ProcessEvent::KillError,
                        &process_data,
                    );
                }
            }
        }
        _error => {
            let reader = stdout_reader.as_ref();
            if reader.err().is_some() {
                process_data
                    .line
                    .push_str(format!("{:?}", reader.err().unwrap()).as_str());
            }
            check_and_trigger_callback(process_req, &ProcessEvent::StartError, &process_data);
        }
    }
    process_data.request = None;
    process_data.reader = None;
}

/// handle pipeline based multiple command lines
fn handle_pipeline(request: &Arc<ProcessRequest>) -> Expression {
    let cmd_line = &request.cmd_line;
    let use_shell = request.use_shell;
    let mut cmd_pipeline;
    if use_shell {
        cmd_pipeline = sh_vector(&cmd_line[0]);
    } else {
        let cli = vec_string_to_osstring(&cmd_line[0]);
        cmd_pipeline = cmd(&cli[0], &cli[1..]);
    }
    if cmd_line.len() > 1 {
        let mut cmd_itr = cmd_line.iter();
        cmd_itr.next();
        for command in cmd_itr {
            if use_shell {
                cmd_pipeline = cmd_pipeline.pipe(sh_vector(&command));
            } else {
                let cli = vec_string_to_osstring(&command);
                cmd_pipeline = cmd_pipeline.pipe(cmd(&cli[0], &cli[1..]));
            }
        }
    }
    cmd_pipeline
}

/// create and run a shell based command, using vector of cmd and arguments
fn sh_vector(command: &Vec<String>) -> Expression {
    let argv = shell_command_argv_vector(command.into());
    cmd(&argv[0], &argv[1..])
}

/// create a shell based command
#[cfg(unix)]
fn shell_command_argv_vector(command: &Vec<String>) -> Vec<OsString> {
    let mut cli: Vec<OsString> = vec_string_to_osstring(command);
    let mut full_args = vec!["/bin/sh".into(), "-c".into()];
    full_args.append(&mut cli);
    full_args
}

/// Prepare shell based command
#[cfg(windows)]
fn shell_command_argv_vector(command: &Vec<String>) -> Vec<OsString> {
    let comspec = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
    let mut cli: Vec<OsString> = vec_string_to_osstring(command);
    let mut full_args = vec![comspec, "/C".into()];
    full_args.append(&mut cli);
    full_args
}

/// convert vector of [`String`] to vector of [`OsString`]
fn vec_string_to_osstring(input: &Vec<String>) -> Vec<OsString> {
    input.iter().map(|x| x.as_str().into()).collect()
}

#[cfg(test)]
mod tests {
    use crate::{ProcessData, ProcessEvent, ProcessRequest};
    use std::sync::Arc;

    #[test]
    pub fn test_using_sh_output_streaming_new_version() {
        let callback = |status: &ProcessEvent, data: &ProcessData| -> Option<bool> {
            match status {
                ProcessEvent::Started => {
                    println!(
                        "Event {:?} | req-id {}  | Pids: {:?}",
                        status,
                        data.request.as_ref().unwrap().request_id,
                        data.child_pids()
                    );
                }
                ProcessEvent::IOData => {
                    println!(
                        "Event {:?} | req-id {} | # {} : {}",
                        status,
                        data.request.as_ref().unwrap().request_id,
                        data.line_number,
                        data.line
                    );

                    //demo how to kill/stop
                    //_ = data.kill();
                    // //or return false if line_number check, to exit the process
                    // if data.line_number == 1 {
                    //     return Some(false);
                    // }
                    // // or return false if a condition is true based on the output, to exit the process
                    // if data.line.contains("Sandy") {
                    //     return Some(false);
                    // }
                }
                other => {
                    if !data.line.is_empty() {
                        println!(
                            "Event {:?} | req-id {} | additional detail(s): {}",
                            other,
                            data.request.as_ref().unwrap().request_id,
                            data.line
                        );
                    } else {
                        println!(
                            "Event {:?} | req-id {}",
                            other,
                            data.request.as_ref().unwrap().request_id
                        );
                    }
                }
            }
            Some(true)
        };

        let request1 = ProcessRequest {
            request_id: 121,
            callback: Some(Arc::new(callback)),
            use_shell: true,
            cmd_line: vec![vec![
                String::from("echo"),
                String::from("stdout"),
                String::from("&"),
                String::from("timeout"),
                String::from("/t"),
                String::from("3"),
                String::from("&"),
                String::from("echo"),
                String::from("stderr"),
                String::from(">&2"),
            ]],
            non_blocking_mode: false,
        };

        let request2 = ProcessRequest {
            request_id: 151,
            callback: Some(Arc::new(callback)),
            use_shell: true,
            cmd_line: vec![vec![
                String::from("echo"),
                String::from("stdout"),
                String::from("&"),
                String::from("timeout"),
                String::from("/t"),
                String::from("2"),
                String::from("&"),
                String::from("echo"),
                String::from("stderr"),
                String::from(">&2"),
            ]],
            non_blocking_mode: true,
        };

        // Non blocking mode
        let result1 = ProcessRequest::start(request1);
        println!("Returned from Start! of non blocking");
        // Blocking mode
        let _result2 = ProcessRequest::start(request2);
        println!("Returned from Start! of blocking");

        //check & wait for the non blocking mode
        if result1.is_some() {
            if result1.as_ref().unwrap().is_ok() {
                println!(
                    "Start - join waiting over in non blocking mode {:?}",
                    result1
                        .unwrap()
                        .unwrap()
                        .join()
                        .expect("Couldn't join on the associated thread")
                );
            } else {
                println!(
                    "Start - Error in non blocking mode {:?}",
                    result1.unwrap().err()
                );
            }
        } else {
            println!("Start - Nothing to wait for!");
        }

        let request3 = ProcessRequest {
            request_id: 161,
            callback: Some(Arc::new(callback)),
            use_shell: true,
            cmd_line: vec![vec![String::from("dir")], vec![String::from("sort")]],
            non_blocking_mode: true,
        };

        println!(
            "test_using_sh_output_streaming, demo pipe-line {:?}",
            //current dir listing and sort the piped output
            ProcessRequest::start(ProcessRequest {
                request_id: 161,
                callback: Some(Arc::new(callback)),
                use_shell: true,
                cmd_line: vec![vec![String::from("dir")], vec![String::from("sort")]],
                non_blocking_mode: true,
            })
        );

        let request4 = ProcessRequest {
            request_id: 171,
            callback: Some(Arc::new(callback)),
            use_shell: true,
            cmd_line: vec![vec![String::from(r#"echo "Sandy" "#)]],
            non_blocking_mode: true,
        };
        println!(
            "test_using_sh_output_streaming , demo double quotes {:?}",
            //current dir listing and sort the piped output
            ProcessRequest::start(request4)
        );

        let request5 = ProcessRequest {
            request_id: 181,
            callback: Some(Arc::new(callback)),
            use_shell: true,
            cmd_line: vec![vec![]],
            non_blocking_mode: true,
        };

        println!(
            "test_using_sh_output_streaming, no arguments means start error {:?}",
            ProcessRequest::start(request5)
        );

        println!(
            "test_using_sh_output_streaming, start calc in windows {:?}",
            ProcessRequest::start(ProcessRequest {
                request_id: 191,
                callback: Some(Arc::new(callback)),
                use_shell: true,
                cmd_line: vec![vec![String::from("calc")]],
                non_blocking_mode: false,
            })
        );

        //
    }
}
