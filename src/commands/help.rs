use std::collections::HashSet;

use serenity::{
    model::{
        channel::Message,
        id::UserId,
    },
    framework::{
        standard::{
            macros::help,
            help_commands,
            Args,
            HelpOptions,
            CommandGroup,
        },
    },
    prelude::*,
};

use crate::Result;

#[help]
pub fn help(
    ctx: &mut Context,
    msg: &Message,
    args: Args,
    opts: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> Result<()> {
    help_commands::with_embeds(ctx, msg, args, opts, groups, owners)
}
