//! /sethome /home /delhome /setwarp /warp /delwarp

use pumpkin_plugin_api::{Server, command::CommandSender};

use crate::{
    state::with,
    util::*,
};

const DEFAULT_HOME: &str = "home";

pub fn set_home(sender: &CommandSender, name: &str) -> Res {
    let p = me(sender)?;
    let id = id_of(&p);
    let pname = p.get_name();
    let loc = loc_of(&p);
    let result = with(|st| {
        let max = st.config.max_homes;
        let d = st.pd(&id, &pname);
        if max > 0 && !d.homes.contains_key(name) && d.homes.len() >= max {
            return Err(max);
        }
        d.homes.insert(name.to_string(), loc);
        st.save();
        Ok(())
    });
    match result {
        Ok(()) => {
            say(sender, &format!("Home '{name}' set."));
            ok()
        }
        Err(max) => fail(&format!("You can only have {max} homes. Delete one with /delhome.")),
    }
}

pub fn go_home(sender: &CommandSender, server: &Server, name: &str) -> Res {
    let p = me(sender)?;
    let id = id_of(&p);
    let loc = with(|st| st.data.players.get(&id).and_then(|d| d.homes.get(name).cloned()));
    let Some(loc) = loc else {
        return fail(&format!("Home '{name}' not found."));
    };
    if !teleport(server, &p, &loc) {
        return fail("The world that home is in is not loaded.");
    }
    say(sender, &format!("Teleported to home '{name}'."));
    ok()
}

/// `/home` with no argument: go to the only home, or list them.
pub fn home_default(sender: &CommandSender, server: &Server) -> Res {
    let p = me(sender)?;
    let id = id_of(&p);
    let names: Vec<String> = with(|st| {
        st.data
            .players
            .get(&id)
            .map(|d| d.homes.keys().cloned().collect())
            .unwrap_or_default()
    });
    match names.len() {
        0 => fail("You have no homes. Use /sethome to create one."),
        1 => go_home(sender, server, &names[0]),
        _ => {
            say(sender, &format!("Homes: {}", names.join(", ")));
            ok()
        }
    }
}

pub fn del_home(sender: &CommandSender, name: &str) -> Res {
    let p = me(sender)?;
    let id = id_of(&p);
    let removed = with(|st| {
        let r = st
            .data
            .players
            .get_mut(&id)
            .and_then(|d| d.homes.remove(name))
            .is_some();
        if r {
            st.save();
        }
        r
    });
    if removed {
        say(sender, &format!("Home '{name}' deleted."));
        ok()
    } else {
        fail(&format!("Home '{name}' not found."))
    }
}

pub fn set_warp(sender: &CommandSender, name: &str) -> Res {
    let p = me(sender)?;
    let loc = loc_of(&p);
    with(|st| {
        st.data.warps.insert(name.to_lowercase(), loc);
        st.save();
    });
    say(sender, &format!("Warp '{name}' created."));
    ok()
}

pub fn go_warp(sender: &CommandSender, server: &Server, name: &str) -> Res {
    let p = me(sender)?;
    let loc = with(|st| st.data.warps.get(&name.to_lowercase()).cloned());
    let Some(loc) = loc else {
        return fail(&format!("Warp '{name}' not found."));
    };
    if !teleport(server, &p, &loc) {
        return fail("The world that warp is in is not loaded.");
    }
    say(sender, &format!("Warped to '{name}'."));
    ok()
}

pub fn list_warps(sender: &CommandSender) -> Res {
    let names: Vec<String> = with(|st| st.data.warps.keys().cloned().collect());
    if names.is_empty() {
        return fail("There are no warps.");
    }
    say(sender, &format!("Warps: {}", names.join(", ")));
    ok()
}

pub fn del_warp(sender: &CommandSender, name: &str) -> Res {
    let removed = with(|st| {
        let r = st.data.warps.remove(&name.to_lowercase()).is_some();
        if r {
            st.save();
        }
        r
    });
    if removed {
        say(sender, &format!("Warp '{name}' deleted."));
        ok()
    } else {
        fail(&format!("Warp '{name}' not found."))
    }
}

pub fn default_home_name() -> &'static str {
    DEFAULT_HOME
}
