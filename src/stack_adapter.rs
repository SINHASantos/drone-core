//! Adapter pattern for transforming stackful synchronous subroutines into
//! fibers.

use fib::{Fiber, FiberState};
use futures::prelude::*;

/// Interface for running synchronous code asynchronously.
pub trait Adapter: Sized + Send + 'static {
  /// Stackful fiber that runs synchronous code.
  type Stack: Fiber<
    Input = In<Self::Cmd, Self::ReqRes>,
    Yield = Out<Self::Req, Self::CmdRes>,
    Return = !,
  >;

  /// See (`Context`)[Context].
  type Context: Context<Self::Req, Self::ReqRes>;

  /// `enum` of all possible commands.
  type Cmd: Send + 'static;

  /// `union` of all possible command results.
  type CmdRes: Send + 'static;

  /// `enum` of all possible requests.
  type Req: Send + 'static;

  /// `union` of all possible request results.
  type ReqRes: Send + 'static;

  /// Session error.
  type Error;

  /// Stack size.
  const STACK_SIZE: usize;

  /// Returns a mutable reference to the stack fiber.
  fn stack(&mut self) -> &mut Self::Stack;

  /// Runs a command `cmd` synchronously.
  fn run_cmd(cmd: Self::Cmd, context: Self::Context) -> Self::CmdRes;

  /// Runs a request `req` asynchronously.
  fn run_req<'a>(
    &'a mut self,
    req: Self::Req,
  ) -> Box<Future<Item = Self::ReqRes, Error = Self::Error> + 'a>;

  /// Returns a future that runs a command `cmd`, and returns its result.
  fn cmd<'a>(
    &'a mut self,
    cmd: Self::Cmd,
  ) -> Box<Future<Item = Self::CmdRes, Error = Self::Error> + 'a> {
    let mut input = In { cmd };
    Box::new(async(static move || loop {
      input = match self.stack().resume(input) {
        FiberState::Yielded(Out::Req(req)) => In {
          req_res: await!(self.run_req(req))?,
        },
        FiberState::Yielded(Out::CmdRes(res)) => break Ok(res),
      }
    }))
  }
}

/// A handler type to make requests.
pub trait Context<Req, ReqRes>: Sized + 'static {
  /// Creates a new `Context`.
  ///
  /// # Safety
  ///
  /// Should be used only inside the code that runs inside the adapter.
  unsafe fn new() -> Self;

  /// Makes a request.
  fn req(&self, req: Req) -> ReqRes;
}

/// Adapter input message.
#[allow(unions_with_drop_fields)]
pub union In<Cmd, ReqRes> {
  /// Command.
  cmd: Cmd,
  /// Request result.
  req_res: ReqRes,
}

/// Adapter output message.
pub enum Out<Req, CmdRes> {
  /// Request.
  Req(Req),
  /// Command result.
  CmdRes(CmdRes),
}

impl<Cmd, ReqRes> In<Cmd, ReqRes> {
  /// Reads the input as a command.
  pub unsafe fn into_cmd(self) -> Cmd {
    self.cmd
  }

  /// Reads the input as a request result.
  pub unsafe fn into_req_res(self) -> ReqRes {
    self.req_res
  }
}
