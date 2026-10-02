//! /back /top /tp /tphere /tpall /tppos /tpa /tpahere /tpaccept /tpdeny /tpacancel

use pumpkin_plugin_api::{Server, command::{CommandSender, ConsumedArgs}};

use crate::{
    state::{TpaRequest, now, with},
    util::*,
};

pub fn back(sender: &CommandSender, server: &Server) -> Res {
    let p = me(sender)?;
    let id = id_of(&p);
    let Some(loc) = with(|st| st.back.get(&id).cloned()) else {
        return fail("You have no previous location to return to.");
    };
    if !teleport(server, &p, &loc) {
        return fail("The world of your previous location is not loaded.");
    }
    say(sender, "Returned to your previous location.");
    ok()
}

pub fn top(sender: &CommandSender) -> Res {
    let p = me(sender)?;
    let (x, _, z) = p.get_position();
    let world = p.get_world();
    let y = world.get_top_block_y(x.floor() as i32, z.floor() as i32);
    p.teleport((x, f64::from(y) + 1.0, z), None, None, world);
    say(sender, "Teleported to the top.");
    ok()
}

/// `/tp <player>` (self -> player) or `/tp <a> <b>` (a -> b).
pub fn tp(sender: &CommandSender, server: &Server, args: &ConsumedArgs, two: bool) -> Res {
    let (mover, dest) = if two {
        (need_player(args, "player")?, need_player(args, "other")?)
    } else {
        (me(sender)?, need_player(args, "player")?)
    };
    let l = loc_of(&dest);
    if !teleport(server, &mover, &l) {
        return fail("That world is not loaded.");
    }
    say(sender, &format!("Teleported {} to {}.", mover.get_name(), dest.get_name()));
    ok()
}

pub fn tphere(sender: &CommandSender, server: &Server, args: &ConsumedArgs) -> Res {
    let p = me(sender)?;
    let other = need_player(args, "player")?;
    let l = loc_of(&p);
    if !teleport(server, &other, &l) {
        return fail("That world is not loaded.");
    }
    say(sender, &format!("Teleported {} to you.", other.get_name()));
    ok()
}

pub fn tpall(sender: &CommandSender, server: &Server) -> Res {
    let p = me(sender)?;
    let my_id = id_of(&p);
    let l = loc_of(&p);
    let mut n = 0;
    for other in server.get_all_players() {
        if id_of(&other) != my_id && teleport(server, &other, &l) {
            n += 1;
        }
    }
    say(sender, &format!("Teleported {n} player(s) to you."));
    ok()
}

pub fn tppos(sender: &CommandSender, args: &ConsumedArgs) -> Res {
    let p = me(sender)?;
    let (Some(x), Some(y), Some(z)) = (
        arg_f64(args, "x"),
        arg_f64(args, "y"),
        arg_f64(args, "z"),
    ) else {
        return fail("Usage: /tppos <x> <y> <z>");
    };
    p.teleport((x, y, z), None, None, p.get_world());
    say(sender, &format!("Teleported to {x:.1}, {y:.1}, {z:.1}."));
    ok()
}

// -------------------------------------------------------------------- tpa

pub fn tpa_request(sender: &CommandSender, args: &ConsumedArgs, here: bool) -> Res {
    let p = me(sender)?;
    let target = need_player(args, "player")?;
    let (my_id, target_id) = (id_of(&p), id_of(&target));
    if my_id == target_id {
        return fail("You can't send a teleport request to yourself.");
    }
    with(|st| {
        let expires = now() + st.config.tpa_timeout_secs;
        st.tpa.insert(
            target_id,
            TpaRequest { from: my_id, here, expires },
        );
    });
    let what = if here { "wants you to teleport to them" } else { "wants to teleport to you" };
    tell(&target, &format!("{} {what}. Type /tpaccept or /tpdeny.", p.get_name()));
    say(sender, &format!("Request sent to {}.", target.get_name()));
    ok()
}

pub fn tpaccept(sender: &CommandSender, server: &Server) -> Res {
    let p = me(sender)?;
    let id = id_of(&p);
    let Some(req) = with(|st| st.tpa.remove(&id)) else {
        return fail("You have no pending teleport requests.");
    };
    if req.expires < now() {
        return fail("That teleport request has expired.");
    }
    let Some(from) = lookup(server, &req.from) else {
        return fail("The player who sent the request is no longer online.");
    };
    let moved = if req.here {
        teleport(server, &p, &loc_of(&from))
    } else {
        teleport(server, &from, &loc_of(&p))
    };
    if !moved {
        return fail("That world is not loaded.");
    }
    tell(&from, &format!("{} accepted your teleport request.", p.get_name()));
    say(sender, "Teleport request accepted.");
    ok()
}

pub fn tpdeny(sender: &CommandSender, server: &Server) -> Res {
    let p = me(sender)?;
    let id = id_of(&p);
    let Some(req) = with(|st| st.tpa.remove(&id)) else {
        return fail("You have no pending teleport requests.");
    };
    if let Some(from) = lookup(server, &req.from) {
        tell(&from, &format!("{} denied your teleport request.", p.get_name()));
    }
    say(sender, "Teleport request denied.");
    ok()
}

pub fn tpacancel(sender: &CommandSender) -> Res {
    let p = me(sender)?;
    let id = id_of(&p);
    let removed = with(|st| {
        let before = st.tpa.len();
        st.tpa.retain(|_, r| r.from != id);
        before - st.tpa.len()
    });
    if removed == 0 {
        return fail("You have no outgoing teleport requests.");
    }
    say(sender, "Teleport request cancelled.");
    ok()
}
