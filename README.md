# process-events-streaming
Rust library for simple yet powerful events based multi process execution(blocking & non-blocking modes) in parallel and with data streaming 

---
 Run a process based on the provided process request.
 Generates various events [`ProcessEvent`] according to the process's life-cycle, process's information and data [`ProcessData`] associated with that event
 # Arguments
 ProcessRequest : [`ProcessRequest`] // A request structure to start a process
 # Return
 For Blocking mode it returns [`None`] and for Non-Blocking mode it will return [`Some(io::Result<JoinHandle<()>>)`], so the caller can join & wait for the process completion if needed!
 # Examples
 ```
// Setup callback (for the process events and data streaming)
//
 use process_events_streaming::{ProcessRequest, ProcessResult, ProcessData, ProcessEvent};
 use std::{thread, time::Duration};
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
                  //now assume we want to exit the process with some data
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



    let request2 = ProcessRequest {
        request_id: 151,
        callback: Some(Arc::new(callback)),
        use_shell: true,
        cmd_line: vec![vec![
            String::from("echo"),
            String::from("stdout"),
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
            println!(
                "Start - join waiting over in non blocking mode {:?}",
                internal_data
            );
        } else {
            internal_data.success = Err(process_result.join_handle.unwrap().err().unwrap());
            println!(
                "Start - Error in non blocking mode {:?}",
                internal_data.success
            );
        }
    } else {
        internal_data = process_result;
    }
    println!("result dump : {:?}", internal_data);

    println!(
     "test_using_sh_output_streaming, start calc in windows {:?}",
     ProcessRequest::start(ProcessRequest {
         request_id: 191,
         callback: Some(Arc::new(callback)),
         use_shell: true,
         cmd_line: vec![vec![String::from("calc")]],
         non_blocking_mode: false,
     }));
 ```

File [`../src/lib.rs`] contains examples in detail under #[test]

Sample output of the process events from the tests

```
running 2 tests
Event Starting | req-id 151 | additional detail(s): Executing in thread-context -> id: ThreadId(2), name: Some("tests::test_using_sh_output_streaming_new_version")
Event Started | req-id 151  | Pids: [14120]
Event IOData | req-id 151 | # 1 : stdout 

Event ExitRequested | req-id 151 | additional detail(s): stdout

Event Exited | req-id 151
Returned from Start! of non blocking
result dump : ProcessResult { join_handle: None, should_exit: Some(true), success: Ok(true), data_vec_str: Some(["I found my hidden data!"]), data_bool: None, data_num: Some(8111981), data_decimal: None }

Event Starting | req-id 105 | additional detail(s): Executing in thread-context -> id: ThreadId(3), name: Some("pes_th_rq_105")
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
test_using_sh_output_streaming, demo pipe-line ()

Event Starting | req-id 106 | additional detail(s): Executing in thread-context -> id: ThreadId(3), name: Some("pes_th_rq_106")
Event Started | req-id 106  | Pids: [25936]
Event IOData | req-id 106 | # 1 : \"Sandy\" 
Event IOEof | req-id 106
Event Exited | req-id 106
test_using_sh_output_streaming, demo double quotes ()

Event StartError | req-id 107 | additional detail(s): "Command line - arguments are unavailable!"
test_using_sh_output_streaming, no arguments means start error ()


```

## License

Licensed under

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)


## Dependency

This library is using a wonderful library 'duct' for low level process handling


