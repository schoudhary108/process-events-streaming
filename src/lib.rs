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
    pub request: Option<Arc<ProcessRequest>>,
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

/// Resulted data received from the process execution
#[derive(Debug)]
pub struct ProcessResult {
    /// In case of non-blocking mode use this to join and wait for the process to complete
    pub join_handle: Option<io::Result<JoinHandle<ProcessResult>>>,
    /// Should exit or not the process based on the custom conditions
    pub should_exit: Option<bool>,
    /// Process execution was successful or not for the desired outcome
    pub success: Result<bool, std::io::Error>,
    /// Date as String vector
    pub data_vec_str: Option<Vec<String>>,
    /// Date as true/false
    pub data_bool: Option<bool>,
    /// Date as numeric value i128
    pub data_num: Option<i128>,
    /// Date as f64 value
    pub data_decimal: Option<f64>,
}

impl ProcessResult {
    fn new() -> Self {
        Self {
            join_handle: None,
            should_exit: None,
            success: Ok(false),
            data_vec_str: None,
            data_bool: None,
            data_num: None,
            data_decimal: None,
        }
    }

    /// set join handle
    fn set_join_handle(&mut self, join_handle: Option<io::Result<JoinHandle<ProcessResult>>>) {
        self.join_handle = join_handle;
    }

    ///set exit and success data
    fn set_exit_flag_and_success(
        &mut self,
        should_exit: bool,
        success: Result<bool, std::io::Error>,
    ) {
        self.should_exit = Some(should_exit);
        self.success = success;
    }
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
    pub callback: Option<Arc<dyn Fn(&ProcessEvent, &ProcessData) -> ProcessResult + 'static>>,
}

impl ProcessRequest {
    /**
     Run a process based on the provided process request which is events based multi process execution(blocking & non-blocking modes) in parallel and with data streaming
     Generates various events [`ProcessEvent`] according to the process's life-cycle, process's information and data [`ProcessData`] associated with that event
     # Arguments
     ProcessRequest : [`ProcessRequest`] // A request structure to start a process
     # Return
     ProcessResult : [`ProcessResult`] // Contains join handle for non-blocking and data variables
     For Blocking mode use join_handle [`Option<io::Result<JoinHandle<ProcessResult>>>`], so the caller can join & wait for the process completion if needed!
     In the callback set the custom data to be retrieved once process execution is over, which will be returned in response of the join call.
     # Examples
     ```
    // Setup callback for the process events and data streaming
    //
    // use process_events_streaming::{ProcessRequest, ProcessResult, ProcessData, ProcessEvent};
    // use std::{thread, time::Duration};
    //      let callback = |status: &ProcessEvent, data: &ProcessData| -> ProcessResult {
    //          match status {
    //              ProcessEvent::Started => {
    //                  println!(
    //                      "Event {:?} | req-id {}  | Pids: {:?}",
    //                      status,
    //                      data.request.as_ref().unwrap().request_id,
    //                      data.child_pids()
    //                  );
    //              }
    //              ProcessEvent::IOData => {
    //                  println!(
    //                      "Event {:?} | req-id {} | # {} : {}",
    //                      status,
    //                      data.request.as_ref().unwrap().request_id,
    //                      data.line_number,
    //                      data.line
    //                  );
    //                  //now assume we want to exit the process with some data
    //                  let mut result = ProcessResult::new();
    //                  result.set_exit_flag_and_success(true, Ok(true));
    //                  result.data_num = Some(8111981);
    //                  result.data_vec_str = Some(vec![String::from("I found my hidden data!")]);
    //                  return result;
    //
    //                  //demo how to kill/stop
    //                  //_ = data.kill();
    //              }
    //              other => {
    //                  if !data.line.is_empty() {
    //                      println!(
    //                          "Event {:?} | req-id {} | additional detail(s): {}",
    //                          other,
    //                          data.request.as_ref().unwrap().request_id,
    //                          data.line
    //                      );
    //                  } else {
    //                      println!(
    //                          "Event {:?} | req-id {}",
    //                          other,
    //                          data.request.as_ref().unwrap().request_id
    //                      );
    //                  }
    //              }
    //          }
    //          ProcessResult::new()
    //      };
    //
    //
    //
    //    let request2 = ProcessRequest {
    //        request_id: 151,
    //        callback: Some(Arc::new(callback)),
    //        use_shell: true,
    //        cmd_line: vec![vec![
    //            String::from("echo"),
    //            String::from("stdout"),
    //            String::from("&"),
    //            String::from("echo"),
    //            String::from("stderr"),
    //            String::from(">&2"),
    //        ]],
    //        non_blocking_mode: true,
    //    };
    //
    //    // non Blocking mode
    //    let process_result = ProcessRequest::start(request2);
    //    println!("Returned from Start! of non blocking");
    //
    //    let mut internal_data = ProcessResult::new();
    //    //check & wait for the non blocking mode
    //    if process_result.join_handle.is_some() {
    //        if process_result.join_handle.as_ref().unwrap().is_ok() {
    //            internal_data = process_result.join_handle.unwrap().unwrap().join().unwrap();
    //            println!(
    //                "Start - join waiting over in non blocking mode {:?}",
    //                internal_data
    //            );
    //        } else {
    //            internal_data.success = Err(process_result.join_handle.unwrap().err().unwrap());
    //            println!(
    //                "Start - Error in non blocking mode {:?}",
    //                internal_data.success
    //            );
    //        }
    //    } else {
    //        internal_data = process_result;
    //    }
    //    println!("result dump : {:?}", internal_data);
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
    pub fn start(process_request: ProcessRequest) -> ProcessResult {
        let request = Arc::new(process_request);
        if request.non_blocking_mode {
            let join_handle = thread::Builder::new()
                .name(format!("pes_th_rq_{}", request.request_id).into())
                .spawn(move || {
                    let response = start_process(request);
                    return response;
                });
            let mut result = ProcessResult::new();
            result.set_join_handle(Some(join_handle));
            return result;
        } else {
            return start_process(request);
        }
    }
}

fn start_process(request: Arc<ProcessRequest>) -> ProcessResult {
    let mut process_result = ProcessResult::new();
    let mut process_data = ProcessData::new();
    process_data.line.clear();
    process_data.request = Some(Arc::clone(&request));
    if request.as_ref().cmd_line.len() == 0 || request.as_ref().cmd_line[0].len() == 0 {
        process_data
            .line
            .push_str(format!("{:?}", "Command line - arguments are unavailable!").as_str());
        return check_and_trigger_callback(&request, &ProcessEvent::StartError, &process_data);
    }
    process_data.line.push_str(
        format!(
            "Executing in thread-context -> id: {:?}, name: {:?}",
            thread::current().id(),
            thread::current().name()
        )
        .as_str(),
    );
    process_result = check_and_trigger_callback(&request, &ProcessEvent::Starting, &process_data);

    let process_req = &request;
    let stdout_reader = handle_pipeline(&request).stderr_to_stdout().reader();
    if stdout_reader.as_ref().is_ok() {
        process_data.reader = Some(stdout_reader.as_ref().unwrap());
    }
    match stdout_reader.as_ref() {
        Ok(stdout_reader) => {
            process_result =
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
                        process_result = check_and_trigger_callback(
                            process_req,
                            &ProcessEvent::IOData,
                            &process_data,
                        );
                        match &process_result {
                            process_result
                                if process_result.should_exit.is_some()
                                    && process_result.should_exit.unwrap() == true =>
                            {
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
    process_result
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

/// check if the callback is registered and if yes then trigger it wi the supplied data
fn check_and_trigger_callback(
    request: &Arc<ProcessRequest>,
    event: &ProcessEvent,
    data: &ProcessData,
) -> ProcessResult {
    if request.callback.as_ref().is_some() {
        return request.callback.as_ref().unwrap()(event, data);
    };
    ProcessResult::new()
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
    use crate::{ProcessData, ProcessEvent, ProcessRequest, ProcessResult};
    use std::{any::Any, fmt::Error, sync::Arc};

    #[test]
    pub fn test_using_sh_output_streaming_new_version() {
        let callback = |status: &ProcessEvent, data: &ProcessData| -> ProcessResult {
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

                    let mut result = ProcessResult::new();
                    result.set_exit_flag_and_success(true, Ok(true));
                    result.data_num = Some(8111981);
                    result.data_vec_str = Some(vec![String::from("I found my hidden data!")]);
                    return result;
                    //demo how to kill/stop
                    //_ = data.kill();
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
            ProcessResult::new()
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

        // non Blocking mode
        let process_result = ProcessRequest::start(request2);
        println!("Returned from Start! of non blocking");

        let mut internal_data = ProcessResult::new();
        //check & wait for the non blocking mode
        if process_result.join_handle.is_some() {
            if process_result.join_handle.as_ref().unwrap().is_ok() {
                internal_data = process_result.join_handle.unwrap().unwrap().join().unwrap();
                println!("Start - join waiting over in non blocking mode");
            } else {
                internal_data.success = Err(process_result.join_handle.unwrap().err().unwrap());
                println!("Start - Error in non blocking mode");
            }
        } else {
            internal_data = process_result;
        }
        println!("result dump : {:?}", internal_data);

        //blocking mode
        let result1 = ProcessRequest::start(request1);
        println!("Returned from Start! of blocking");

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
