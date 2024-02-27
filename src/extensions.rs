use std::process::Command;

use tokio::process::Command as TokioCommand;

pub trait Also {
    fn also(&mut self, op: impl FnOnce(&mut Self)) -> &mut Self;
}

impl<T> Also for T {
    fn also(&mut self, op: impl FnOnce(&mut Self)) -> &mut Self {
        op(self);
        self
    }
}

pub trait Nop {
    fn nop(&mut self);
}

impl<T> Nop for T {
    fn nop(&mut self) {
        // do nothing
    }
}

pub trait ToTokioCommand {
    fn tokio(self) -> TokioCommand;
}

impl ToTokioCommand for Command {
    fn tokio(self) -> TokioCommand {
        TokioCommand::from(self)
    }
}
