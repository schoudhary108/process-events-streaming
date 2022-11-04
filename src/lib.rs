use duct::{cmd, Expression, ReaderHandle};
use std::ffi::OsString;
use std::io::{BufRead, BufReader};

use std::io;

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
    /// Error occured while starting the process itself
    StartError,
    /// Process started but error occured during reading the output data
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
    /// A error occured while killing/stopping the process
    KillError,
}

/// Various fields related to the process
///
pub struct ProcessData<'a> {
    /// A unique request-id / session-id
    pub request_id: u32,
    /// List of child pids
    pub pids: Vec<u32>,
    /// Process's STDOUT & STDERR output's line number
    pub line_number: i64,
    /// Process's STDOUT & STDERR output's line
    pub line: String,
    /// Internal reader handle for managing the process
    reader: Option<&'a ReaderHandle>,
    /// Callback to register for the process events
    callback: Option<&'a dyn Fn(&ProcessEvent, &ProcessData) -> Option<bool>>,
}

impl ProcessData<'_> {
    /// Create a new instance of the [`ProcessData`]
    ///
    pub fn new() -> Self {
        Self {
            request_id: 0,
            pids: vec![],
            line_number: 0,
            line: String::new(),
            reader: None,
            callback: None,
        }
    }
    /// Kill the running process
    ///
    pub fn kill(&self) -> io::Result<()> {
        Ok(if self.reader.is_some() {
            if self.callback.is_some() {
                self.callback.unwrap()(&ProcessEvent::KillRequested, &self);
            }
            return self.reader.as_ref().unwrap().kill();
        })
    }

    /// Get the list of child pids
    pub fn child_pids(&mut self) -> &Vec<u32> {
        if self.reader.is_some() {
            self.pids = self.reader.as_ref().unwrap().pids();
        }
        self.pids.as_ref()
    }
}

/// check if the callback is registed and if yes then trigger it wi the supplied data
fn check_and_trigger_callback(
    callback: &Option<&dyn Fn(&ProcessEvent, &ProcessData) -> Option<bool>>,
    event: &ProcessEvent,
    data: &ProcessData,
) -> Option<bool> {
    if callback.is_some() {
        return callback.unwrap()(event, data);
    };
    Some(true)
}

/**
Run a process based on the provided arguments.
Generates various events [`ProcessEvent`] according to the process's life-cycle, process's information and data [`ProcessData`] associated with that event

# Arguments
request_id : [`u32`] // custom unique numeric id to relate the various callbacks for a particular process execution session

use_shell : [`bool`] // use shell mode or direct executable path based execution

cmd_line : [`Vec<Vec<String>>`] // Vector of commandline alog with arguments. For a single command line one vector element is enough,
for the pipe lines use case output of one to provide to the next use Vector of Commandlines.

callback : [`Option<&dyn Fn(&ProcessEvent, &ProcessData) -> Option<bool>>`] // register callback to get various events and process output, for no callbacks use None


# Examples

```
//using shell mode, prints hi and waits for 3 seconds. Events are sent to provided callback

//let callback = |status: &ProcessEvent, data: &ProcessData| -> Option<bool> {};
// run_process(
//     102,
//     true,
//     vec![vec![
//         String::from("echo"),
//         String::from("hi"),
//         String::from("&"),
//         String::from("timeout"),
//         String::from("/t"),
//         String::from("3"),
//     ]],
//     Some(&callback)
// );
//
// using cmd mode, starts calculator application in windows
// run_process(101, false, vec![vec![String::from("calc")]], None);
//
// running process using a thread
// _ = thread::spawn(move || {
//            run_process(101, false, vec![vec![String::from("calc")]], None);
//     });

```
*/
pub fn run_process(
    request_id: u32,
    use_shell: bool,
    cmd_line: Vec<Vec<String>>,
    callback: Option<&dyn Fn(&ProcessEvent, &ProcessData) -> Option<bool>>,
) {
    let mut process_data = ProcessData::new();
    process_data.line.clear();
    process_data.request_id = request_id;
    process_data.callback = callback;
    if cmd_line.len() == 0 || cmd_line[0].len() == 0 {
        process_data
            .line
            .push_str(format!("{:?}", "Commandline - arguments are unavailable!").as_str());
        check_and_trigger_callback(&callback, &ProcessEvent::StartError, &process_data);
        return;
    }

    check_and_trigger_callback(&callback, &ProcessEvent::Starting, &process_data);

    let handle = handle_pipeline(cmd_line, use_shell);
    let reader = &handle.stderr_to_stdout().reader();

    match reader {
        Ok(reader) => {
            process_data.reader = Some(reader);
            process_data.pids = reader.pids();
            check_and_trigger_callback(&callback, &ProcessEvent::Started, &process_data);
            let mut buffer_reader = BufReader::new(reader);
            loop {
                process_data.line.clear();
                let result = buffer_reader.read_line(&mut process_data.line);
                match result {
                    Ok(result) if result == 0 => {
                        check_and_trigger_callback(&callback, &ProcessEvent::IOEof, &process_data);
                        break;
                    }
                    Ok(_result) => {
                        process_data.line_number += 1;
                        match check_and_trigger_callback(
                            &callback,
                            &ProcessEvent::IOData,
                            &process_data,
                        ) {
                            Some(value) if value == false => {
                                check_and_trigger_callback(
                                    &callback,
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
                            &callback,
                            &ProcessEvent::IOError,
                            &process_data,
                        );
                        break;
                    }
                }
            }
            process_data.line.clear();
            let exit_result = reader.kill();

            match exit_result {
                Ok(_) => {
                    check_and_trigger_callback(&callback, &ProcessEvent::Exited, &process_data);
                }
                Err(_) => {
                    check_and_trigger_callback(&callback, &ProcessEvent::KillError, &process_data);
                }
            }
        }
        _error => {
            let reader = reader.as_ref();
            if reader.err().is_some() {
                process_data
                    .line
                    .push_str(format!("{:?}", reader.err().unwrap()).as_str());
            }
            check_and_trigger_callback(&callback, &ProcessEvent::StartError, &process_data);
        }
    }
    process_data.callback = None;
    process_data.reader = None;
}

/// handle pipeline based multiple commandlines
fn handle_pipeline(cmd_line: Vec<Vec<String>>, use_shell: bool) -> Expression {
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
    use crate::{run_process, ProcessData, ProcessEvent};
    use std::{thread, time::Duration};

    #[test]
    pub fn test_using_sh_output_streaming() {
        let callback = |status: &ProcessEvent, data: &ProcessData| -> Option<bool> {
            match status {
                ProcessEvent::Started => {
                    println!(
                        "Event {:?} | req-id {}  | Pids: {:?}",
                        status, data.request_id, data.pids
                    );
                }
                ProcessEvent::IOData => {
                    println!(
                        "Event {:?} | req-id {} | # {} : {}",
                        status, data.request_id, data.line_number, data.line
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
                            "Event {:?} | req-id {} | addational detail(s): {}",
                            other, data.request_id, data.line
                        );
                    } else {
                        println!("Event {:?} | req-id {}", other, data.request_id);
                    }
                }
            }
            Some(true)
        };

        _ = thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            println!(
                "test_using_sh_output_streaming in a thread, no arguments means error {:?}",
                run_process(104, true, vec![vec![]], Some(&callback))
            );
        });

        println!(
            "test_using_sh_output_streaming pipe-line {:?}",
            //current dir listing and sort the piped output
            run_process(
                105,
                true,
                vec![vec![String::from("dir")], vec![String::from("sort"),]],
                Some(&callback)
            )
        );

        println!(
            "test_using_sh_output_streaming double quotes {:?}",
            //current dir listing and sort the piped output
            run_process(
                106,
                true,
                vec![vec![String::from(r#"echo "Sandy" "#),]],
                Some(&callback)
            )
        );

        println!(
            "test_using_sh_output_streaming no arguments means error {:?}",
            run_process(107, true, vec![vec![]], Some(&callback))
        );
    }

    #[test]
    pub fn test_using_cmd_output_streaming() {
        println!(
            "test_using_cmd_output_streaming {:?}",
            run_process(108, false, vec![vec![String::from("calc")]], None)
        );
    }
}
