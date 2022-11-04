mod process_events_streaming;

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use crate::process_events_streaming::process::{run_process, ProcessData, ProcessEvent};
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
