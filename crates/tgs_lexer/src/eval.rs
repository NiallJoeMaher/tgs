/*
Citation:
```
@inproceedings{eval,
    title = "Nut nsh - A POSIX shell",
    author = "Seiya Nuta",
    year = "2023",
    url = "https://github.com/nuta/nsh/blob/main/src/eval.rs",
}
```
*/

use std::process::ExitStatus;

use tgs_utils::{run_external_command, JobManager, Output, Process, ProcessGroup, Stdin};

use crate::{ast, PosixError};

pub struct Os {
    _job_manager: JobManager,
    /// Exit status of last command executed.
    _last_exit_status: ExitStatus,
}

pub fn run_job(
    job_manager: &mut JobManager,
    procs: Vec<Box<dyn Process>>,
    pgid: Option<u32>,
    foreground: bool,
) -> Result<(), PosixError> {
    let proc_group = ProcessGroup {
        id: pgid,
        processes: procs,
        foreground,
    };

    let is_foreground = proc_group.foreground;
    let job_id = job_manager.create_job("", proc_group);

    if is_foreground {
        job_manager
            .put_job_in_foreground(Some(job_id), false)
            .map_err(|e| PosixError::Job(e))?;
    } else {
        job_manager
            .put_job_in_background(Some(job_id), false)
            .map_err(|e| PosixError::Job(e))?;
    }
    Ok(())
}

/// Returns group of processes and also the pgid if it has one
pub fn eval_command(
    job_manager: &mut JobManager,
    cmd: &ast::Command,
    stdin: Option<Stdin>,
    stdout: Option<Output>,
) -> Result<(Vec<Box<dyn Process>>, Option<u32>), PosixError> {
    match cmd {
        ast::Command::Simple {
            assigns: _,
            redirects: _,
            args,
        } => {
            let mut args_it = args.iter();
            let program = args_it.next().unwrap();
            let args = args_it.collect::<Vec<_>>();

            let proc_stdin = stdin.unwrap_or(Stdin::Inherit);
            let proc_stdout = stdout.unwrap_or(Output::Inherit);

            let (proc, pgid) = match run_external_command(
                program,
                &args,
                proc_stdin,
                proc_stdout,
                Output::Inherit,
                None,
            ) {
                Ok((proc, pgid)) => (proc, pgid),
                Err(e) => match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        return Err(PosixError::CommandNotFound(program.clone()))
                    }
                    _ => return Err(PosixError::Eval(e.into())),
                },
            };
            Ok((vec![proc], pgid))
        }
        ast::Command::Pipeline(a_cmd, b_cmd) => {
            let (mut a_procs, _a_pgid) =
                eval_command(job_manager, a_cmd, stdin, Some(Output::CreatePipe))?;
            let (b_procs, b_pgid) = eval_command(
                job_manager,
                b_cmd,
                a_procs.last_mut().unwrap().stdout(),
                stdout,
            )?;
            a_procs.extend(b_procs);
            Ok((a_procs, b_pgid))
        }
        ast::Command::AsyncList(a_cmd, b_cmd) => {
            // TODO double check stdin and stdout
            let (procs, pgid) = eval_command(job_manager, a_cmd, None, None)?;
            run_job(job_manager, procs, pgid, false)?;

            if let Some(b_cmd) = b_cmd {
                eval_command(job_manager, b_cmd, None, None)
            } else {
                Ok((vec![], None))
            }
        }
        ast::Command::None => Ok((vec![], None)),
        _ => todo!(),
    }
}
