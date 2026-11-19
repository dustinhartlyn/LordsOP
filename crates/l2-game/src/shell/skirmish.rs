//! Settings and musters for the built-in skirmish roster.

use l2_formats::{troops, Battles, Troops};

const FIELDS: [(&str, &str, &str); troops::ROWS] = [
    ("Alder Ford", "The crossing at Alder Ford", "The ferrymen have cut their ropes. Hold the wet ground until the last of the wagons reaches the north bank."),
    ("Reedbank", "Stand among the reeds", "Two banners rise from the riverside haze. Keep the men together until the two hosts can see each other."),
    ("North Common", "Muster on the common", "The common is broad enough for both armies to dress their lines. Neither company will have anywhere to hide."),
    ("Hedge Road", "The road through the hedges", "A string of broken hedges divides the old drovers' road. Make use of whatever cover remains."),
    ("Thorn Pasture", "The thorn pasture", "The shepherds have gone and left their gates open. At noon the pasture will belong to whoever can hold it."),
    ("East Fen", "A march into the fen", "There is no easy way across this wet country. Seek a clear line before committing the slower companies."),
    ("Low Barrow", "The watch at Low Barrow", "A watchman saw the rival banners before first light. The companies came armed and ready to settle the matter."),
    ("Haycross", "The dispute at Haycross", "The tithe wagons stopped at a disputed boundary. The two escorts draw up with the bales already unloaded."),
    ("Dun Meadow", "Banners in the dun meadow", "A valley of faded grasses offers room for a wide march. Protect the ends of the line from a turning force."),
    ("Shale Run", "The road to Shale Run", "The last grain convoy waits beyond the far hills. A battle here decides who collects it."),
    ("Rook Hill", "On the slopes of Rook Hill", "The hills and scattered trees break up the country. Keep reserves where they can reach the front."),
    ("Slate Ridge", "The meeting at Slate Ridge", "A cold wind comes down from the ridge. The scouts expect a heavier force than usual on the other side."),
    ("Grey Coomb", "Two hosts in Grey Coomb", "Both armies have answered the challenge with large musters. The skirmishers will not settle this on their own."),
    ("Pine Hollow", "In the shade of the pines", "The tree line hid the enemy march for a day. It covers neither host now."),
    ("Crow Fell", "The fall of Crow Fell", "There is little but stony ground for a mile in either direction. Place the archers before the lines come together."),
    ("Horn Vale", "Horn calls in the valley", "The hunters' horns have given way to signals from the vanguard. The scouts report a large enemy reserve."),
    ("Ash Heath", "Twin banners on Ash Heath", "The heath has seen no rain in weeks. Dust will rise long before the furthest company comes into view."),
    ("Wren Gap", "The narrow way at Wren Gap", "Two scouts reached the gap at dawn. The rival hosts soon followed them."),
    ("Flint Weald", "Muster at Flint Weald", "The smiths have sent enough spare arms to keep both armies in the field. No one plans to give up the weald without a fight."),
    ("West Tor", "The last march to West Tor", "Both commanders have called up their best companies. The field is the last strong position before the coast."),
    ("Birch Motte", "The stakes of Birch Motte", "Fresh timber crowns a low earthen bank. Break the ring or protect the road through its gate."),
    ("Juniper Yard", "The Juniper Yard palisade", "The people of the outer farms are sheltering behind the timber wall. The defenders have only a short time to make ready."),
    ("Hawk Roost", "The hill fort of Hawk Roost", "The fort overlooks an old hunting road. The wall will not defend itself; keep men moving to threatened points."),
    ("Oak Ward", "The siege of Oak Ward", "The gate has held since the first timber was set. A proper train of engines is coming to test it."),
    ("Moorgate", "The garrison at Moorgate", "The little stone ward controls a hard road across the moor. Its garrison has seen the approaching army from the west tower."),
    ("Tintagel Ford", "Tintagel Ford under siege", "The toll house has become a way station for the army. Neither side intends to leave the ford to its enemy."),
    ("Hearthwall", "The two walls of Hearthwall", "A second line of stone shelters the old kitchens. The cooks remain inside with the garrison."),
    ("Ivory Ward", "The old ward of ivory stone", "The high walls still show the marks of earlier sieges. The new garrison hopes its repairs will hold."),
    ("North Spire", "Storming the northern spire", "The spire can be seen a day's march away. The engineers are finally in range of the lower walls."),
    ("Willow Keep", "The willows below the keep", "A belt of pollarded trees hides the siege camp. The defenders have had time to lay supplies in the ward."),
    ("Marshwatch", "The gates of Marshwatch", "The fortress commands the only good road through the marsh. The commander cannot afford the loss of its gate."),
    ("Tern Hold", "Tern Hold against the tide", "Wind from the coast whips the banners on the parapet. Supplies have stopped arriving by sea."),
    ("Blackbridge", "The watch over Blackbridge", "A dark stone span brings the road directly to the walls. The guards saw the largest siege train of the season crossing it."),
    ("Cairn Castle", "Cairn Castle on the border", "The border lords have argued over this fortress for a generation. Let the engines put an end to the argument."),
    ("Crown Citadel", "The last citadel of the crown", "The largest muster of the year has reached the citadel. Its defenders have no land left to fall back upon."),
];

pub(super) fn battles() -> Battles {
    let mut text = String::new();
    for (name, heading, briefing) in FIELDS {
        text.push_str(name);
        text.push('\n');
        text.push_str(heading);
        text.push('\n');
        text.push_str(briefing);
        text.push('\n');
    }
    Battles::parse(text.as_bytes())
}

pub(super) fn troops() -> Troops {
    let mut out = Troops {
        advantage: [0; troops::ROWS],
        rows: Box::new([[[[0; troops::SLOTS]; troops::DIFFS]; troops::SIDES]; troops::ROWS]),
    };
    for (row, sides) in out.rows.iter_mut().enumerate() {
        let variant = row % 5;
        let scale = (row / 5 + 1) as i16;
        let mut attack = match variant {
            0 => [64, 13, 20, 12, 18, 16, 4, 0, 0, 0, 0],
            1 => [42, 20, 16, 18, 27, 9, 6, 0, 0, 0, 0],
            2 => [80, 8, 30, 10, 18, 14, 5, 0, 0, 0, 0],
            3 => [38, 20, 21, 26, 11, 8, 7, 0, 0, 0, 0],
            _ => [46, 17, 15, 12, 21, 18, 10, 0, 0, 0, 0],
        };
        let mut defend = match variant {
            0 => [58, 12, 15, 11, 30, 17, 4, 0, 0, 0, 0],
            1 => [49, 13, 21, 19, 13, 19, 6, 0, 0, 0, 0],
            2 => [55, 11, 15, 11, 32, 15, 6, 0, 0, 0, 0],
            3 => [44, 20, 15, 10, 24, 15, 8, 0, 0, 0, 0],
            _ => [54, 12, 20, 17, 14, 16, 10, 0, 0, 0, 0],
        };
        for n in 0..troops::SIEGE_SLOT {
            attack[n] += scale * (5 - n.min(4) as i16);
            defend[n] += scale * (4 - n.min(3) as i16);
        }
        if row >= 20 {
            attack[7] = 1 + (row - 20) as i16 / 4;
            attack[8] = (row - 20) as i16 / 5;
            attack[9] = 1 + (row - 20) as i16 / 6;
            defend[5] += 18;
            defend[10] = (row - 20) as i16 / 3;
            out.advantage[row] = 2 + (row - 20) as i16 / 4;
        }
        for (side, normal) in [attack, defend].into_iter().enumerate() {
            for (diff, p) in troops::SCALE_PERCENT.into_iter().enumerate() {
                for (slot, n) in normal.into_iter().enumerate() {
                    let delta = (n as i32 * (p - 100) / 100) as i16;
                    sides[side][diff][slot] = if slot < troops::SIEGE_SLOT { n + delta } else { n };
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_rosters_cover_all_field_and_siege_presets() {
        let battles = battles();
        let roster = troops();
        assert_eq!(battles.count(), troops::ROWS);
        for row in 0..troops::ROWS {
            assert!(!battles.entry(row * 3 + 2).is_empty());
            for side in 0..troops::SIDES {
                let counts = roster.counts(row, side, troops::NORMAL);
                assert!(counts[..troops::SIEGE_SLOT].iter().sum::<i16>() > 0);
                assert!(counts[troops::SIEGE_SLOT..].iter().all(|&n| n <= troops::SIEGE_MAX));
            }
        }
        assert_eq!(roster.counts(0, 0, troops::NORMAL)[7..], [0; 4]);
        assert!(roster.counts(20, 0, troops::NORMAL)[7] > 0);
    }
}
