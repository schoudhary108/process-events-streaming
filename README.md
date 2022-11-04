# process-events-streaming
Rust library for simple yet powerful events based process execution and data streaming 

---
Use function 'run_process' to run a process based on the provided arguments.
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
## License

Licensed under

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)


## Dependency

This library is using a wondeful library 'duct' for low level process handling


