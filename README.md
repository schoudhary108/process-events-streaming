# process-events-streaming
Rust library for simple yet powerful events based process execution and data streaming 

---
Use function 'run_process' to run a process based on the provided arguments.
Generates various events [`ProcessEvent`] according to the process's life-cycle, process's information and data [`ProcessData`] associated with that event

# Arguments
request_id : [`u32`] // custom unique numeric id to relate the various callbacks for a particular process execution session

use_shell : [`bool`] // use shell mode or direct executable path based execution

cmd_line : [`Vec<Vec<String>>`] // Vector of command line along with arguments. For a single command line one vector element is enough,
for the pipe lines use case output of one to provide to the next use Vector of Command lines.

callback : [`Option<&dyn Fn(&ProcessEvent, &ProcessData) -> Option<bool>>`] // register callback to get various events and process output, for no callbacks use None


# Examples

```
//using shell mode, prints hi and waits for 3 seconds. Events are sent to provided callback

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
                    //
                    // // or return false if a condition is true based on the output, to exit the process
                    // if data.line.contains("Sandy") {
                    //     return Some(false);
                    // }
                }

                other => {
                    if !data.line.is_empty() {
                        println!(
                            "Event {:?} | req-id {} | additional detail(s): {}",
                            other, data.request_id, data.line
                        );
                    } else {
                        println!("Event {:?} | req-id {}", other, data.request_id);
                    }
                }
            }
            Some(true)
 };
 ```
 ```       
 run_process(
     102,
     true,
     vec![vec![
         String::from("echo"),
         String::from("hi"),
         String::from("&"),
         String::from("timeout"),
         String::from("/t"),
         String::from("3"),
     ]],
     Some(&callback)
 );

 using cmd mode, starts calculator application in windows
 run_process(101, false, vec![vec![String::from("calc")]], None);

 running process using a thread
 _ = thread::spawn(move || {
            run_process(101, false, vec![vec![String::from("calc")]], None);
    });

```

File [`../src/lib.rs`] contains examples in detail

Sample output of the process events from the tests

```
running 2 tests

Event Starting | req-id 105
Event Started | req-id 105  | Pids: [10332, 864]
Event IOData | req-id 105 | # 1 : 
Event IOData | req-id 105 | # 2 :
Event IOData | req-id 105 | # 3 :                5 Dir(s)  679,760,900,096 bytes free
Event IOData | req-id 105 | # 4 :                7 File(s)          8,018 bytes
Event IOData | req-id 105 | # 5 :  Directory of C:\process-events-streaming
Event IOData | req-id 105 | # 6 : 04-11-2022  01:53 PM               210 rustfmt.toml
Event IOData | req-id 105 | # 7 : 04-11-2022  04:58 PM               682 Cargo.toml
Event IOData | req-id 105 | # 8 : 04-11-2022  05:09 PM    <DIR>          .github
Event IOData | req-id 105 | # 9 : 04-11-2022  05:09 PM    <DIR>          src
Event IOData | req-id 105 | # 10 : 04-11-2022  09:58 PM                91 .gitconfig
Event IOData | req-id 105 | # 11 : 04-11-2022  09:59 PM             1,091 LICENSE
Event IOData | req-id 105 | # 12 : 04-11-2022  10:11 PM               344 .gitignore
Event IOData | req-id 105 | # 13 : 04-11-2022  10:45 PM             3,600 README.md
Event IOEof | req-id 105
Event Exited | req-id 105
test_using_sh_output_streaming pipe-line ()

Event Starting | req-id 106
Event Started | req-id 106  | Pids: [25936]
Event IOData | req-id 106 | # 1 : \"Sandy\" 
Event IOEof | req-id 106
Event Exited | req-id 106
test_using_sh_output_streaming double quotes ()

Event StartError | req-id 107 | additional detail(s): "Command line - arguments are unavailable!"
test_using_sh_output_streaming no arguments means error ()

test tests::test_using_sh_output_streaming ... ok
test_using_cmd_output_streaming ()

test tests::test_using_cmd_output_streaming ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
```

## License

Licensed under

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)


## Dependency

This library is using a wonderful library 'duct' for low level process handling


