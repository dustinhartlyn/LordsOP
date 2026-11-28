//! Standalone message copy. Group numbers are protocol keys, not player-facing text.
//! Keep the installed language table and existing campaign/tutorial transcriptions first.

use crate::message::{category, group, Record, Shape};
use crate::screen::Ctx;
use crate::shell::ShellAssets;
use l2_kingdom::diplomacy::group as diplo;
use l2_kingdom::report;
use l2_kingdom::victory::{MSG_AI_CROWNED, MSG_AI_ELIMINATED, MSG_DEFEAT, MSG_VICTORY};

pub(super) fn words<'a>(shell: &'a ShellAssets, group: usize, index: usize, fallback: &'a str) -> &'a str {
    nonblank(shell.text(group, index)).unwrap_or(fallback)
}

pub(super) fn nonblank(text: &str) -> Option<&str> {
    (!text.trim().is_empty()).then_some(text)
}

pub(crate) fn title<'a>(ctx: &'a Ctx, record: &Record) -> &'a str {
    let group = record.group;
    let installed = ctx.assets.shell.text(usize::from(group), 0);
    // FREE is a stock lead-in marker on groups that used the county's name instead.
    if let Some(text) = nonblank(installed).filter(|text| !matches!(text.trim(), "FREE" | "FREE.")) {
        return text;
    }
    let recorded = match record.shape() {
        Shape::Help => crate::message::help::TEXT,
        Shape::Paragraphs(_) => crate::tip::TEXT,
        _ => &[],
    };
    if let Some((_, lines)) = recorded.iter().find(|(id, lines)| *id == group && lines.len() > 1) {
        if let Some(text) = lines.first().and_then(|line| nonblank(line)) {
            return text;
        }
    }
    copy(record).0
}

/// The displayed body, including the selected voice variant or the player's own note.
pub fn body(ctx: &Ctx, record: &Record) -> String {
    body_text(ctx, record).to_owned()
}

pub(crate) fn body_text<'a>(ctx: &'a Ctx, record: &Record) -> &'a str {
    let shape = record.shape();
    if shape == Shape::DiplomaticLetter && matches!(record.group, 0xF6 | 0xF7) {
        if let Some(letter) = ctx.game.messages.letters.get(usize::from(record.from)).and_then(|s| nonblank(s)) {
            return letter;
        }
    }
    let index = match shape {
        // These layouts do not store a text variant (e.g. garrison stores the army's slot).
        Shape::Capture | Shape::Event | Shape::Garrison | Shape::DiplomaticLetter | Shape::Help
        | Shape::Paragraphs(_) | Shape::Tip => 1,
        // Voice variants are bytes. Do not pass an arbitrary imported integer to the
        // language table's linear string lookup.
        _ => match u8::try_from(record.variant) {
            Ok(variant) => usize::from(variant) + 1,
            Err(_) => return copy(record).1,
        },
    };
    body_at(ctx, record, index)
}

pub(super) fn body_at<'a>(ctx: &'a Ctx, record: &Record, index: usize) -> &'a str {
    let group = record.group;
    let shell = &ctx.assets.shell;
    if let Some(text) = nonblank(shell.text(usize::from(group), index)) {
        return text;
    }
    let transcribed = match record.shape() {
        Shape::Help => crate::message::help::TEXT.iter().find(|(id, _)| *id == group)
            .and_then(|(_, lines)| lines.get(index)).copied().unwrap_or_default(),
        Shape::Paragraphs(_) => crate::tip::transcribed(group, index),
        _ => crate::arrival::transcribed(group, index),
    };
    if index != 1 && matches!(record.shape(), Shape::Help | Shape::Paragraphs(_)) {
        nonblank(transcribed).unwrap_or_default()
    } else {
        nonblank(transcribed).unwrap_or_else(|| copy(record).1)
    }
}

fn copy(record: &Record) -> (&'static str, &'static str) {
    match record.group {
        // Refusals from map, army, castle, options and battle controls.
        110 => ("Siege order unavailable", "You can give this order during a siege, not in a field battle."),
        111 => ("No drawbridge", "There is no drawbridge to lower at this castle."),
        group::NO_RIGHT_OF_RULE => ("County outside your rule", "You can manage towns and merchants only in counties you govern."),
        group::CANNOT_DISBAND => ("Army cannot disband here", "Return this army to the county where it was raised before disbanding it."),
        group::CASTLE_ALREADY => ("Castle already built", "The county already has the selected type of castle. Choose a stronger design to upgrade it."),
        group::CASTLE_STRONGER => ("Castle cannot be downgraded", "The selected design is weaker than the existing castle. Choose a stronger design to upgrade it."),
        group::UNDER_FIFTY => ("Army too small", "An army needs at least 50 troops. Add more to the muster or adjust the split."),
        group::ALREADY_MARCHED => ("Army has already marched", "An army can only be split before it moves. Try again at the start of the next season."),
        157 => ("Drawbridge already lowered", "The castle's drawbridge is already down."),
        164 => ("Castle under construction", "This castle cannot take a garrison until the building work is complete."),
        165 => ("Castle in ruins", "The castle cannot take a garrison while it is ruined."),
        166 => ("Not enough room in the castle", "The garrison cannot take this whole army."),
        167 => ("Mercenary companies cannot combine", "These armies both contain mercenaries. Keep the companies in separate armies."),
        group::ZERO_MEN => ("No troops selected", "Assign troops in the armoury before raising an army."),
        group::NOWHERE_TO_STAND => ("Army could not be raised", "There is no free place in this county for a new army to muster."),
        260 => ("Display mode unchanged", "Windowed mode cannot be enabled from this menu on the current display."),
        274 => ("Combined army too large", "The two armies would exceed the troop limit for a single army."),
        275 => ("Castle already besieged", "Another army is already laying siege to this castle."),
        281 => ("Siege engines needed", "Build siege engines before launching an assault on the castle."),
        group::GARRISON_FULL => ("Detachment too large", "The selected troops exceed the castle's available garrison space. Send a smaller detachment."),
        283 => ("Garrison full", "This castle has no room for additional troops."),
        284 => ("Castle under construction", "You cannot besiege an unfinished castle."),
        285 => ("Castle has no defenders", "There is no garrison to besiege. Engage enemy forces in the county instead."),
        286 => ("A garrison holds the county", "Besiege the occupied castle before trying to take the county."),
        288 => ("Retreating army lost", "Too few troops remained after the retreat for this army to stay in the field."),
        289 => ("Garrison under siege", "You cannot add troops to a castle while it is under siege."),
        // Arrival, occupation, county greetings and campaign endings.
        114 => ("County changes hands", "A rival has taken this county from another ruler."),
        115 => ("County lost", "Enemy troops have taken the county town."),
        116 => ("Independent county occupied", "A rival has taken control of this formerly independent county."),
        117..=122 | 124..=126 => ("County captured", "Your army has taken this county."),
        123 => ("One county left", "Your army has taken this county. One more stands between you and the crown."),
        129 => ("County too far to govern", "The county is too far from your lands for you to hold it."),
        130 => ("County appeals for help", "The people ask you to visit their town and help them."),
        131 => ("A county welcomes your army", "The people welcome your army as it passes through."),
        132 => ("County asks your army to move on", "The people ask that your army pass quickly through their lands."),
        133 => ("County demands your withdrawal", "The people demand that your army leave their lands."),
        134 => ("County protests the incursion", "The people condemn the entry of your army into their lands."),
        MSG_AI_ELIMINATED if record.variant >= 4 => ("Rival realm defeated", "This lord's bid for the crown is over."),
        MSG_AI_ELIMINATED => ("Rival realm defeated", "This rival has been eliminated from the contest for the crown."),
        MSG_AI_CROWNED => ("A rival takes the crown", "A rival has secured the crown."),
        MSG_DEFEAT => ("Campaign lost", "Your realm has been eliminated from the contest for the crown."),
        MSG_VICTORY => ("The crown is yours", "You have won the campaign and laid claim to the crown."),
        // Weather, unrest, taxes, supply and troop reports.
        report::MSG_COUNTY_SECEDED => ("County declares independence", "Cut off from the heart of your realm, this county has declared independence."),
        report::MSG_LANDS_DIVIDE => ("Your lands have divided", "Counties cut off from the heart of your realm have declared independence."),
        report::MSG_DROUGHT => ("Drought", "Dry weather has ruined a field in this county. Check its grain supply before setting rations."),
        report::MSG_FLOODING => ("Flooding", "Floodwater has ruined a field in this county. Check its grain supply before setting rations."),
        report::MSG_UNREST_WARNING => ("Unrest warning", "Your people are unhappy. Review food rations and taxes before discontent spreads."),
        150 => ("Murmurs of unrest", "Discontent is taking hold. Review the county's food rations and tax rate."),
        151 => ("Unrest is rising", "Discontent in the county has grown. Check what is driving down happiness."),
        152 => ("County in uproar", "The county is close to open revolt. Review its happiness report and address the causes of unrest."),
        153 => ("Open revolt", "The county's discontent has reached open revolt."),
        154 => ("Revolt spreads", "Rebellion in a neighbouring county is spreading unrest here."),
        155 => ("Trouble spreads", "Unrest in a neighbouring county is troubling the people here."),
        156 => ("People grow uneasy", "Your people have heard of trouble in a neighbouring county."),
        160 => ("Mercenaries desert", "The treasury could not cover army wages, so your mercenary companies have left."),
        161 => ("Supply convoy lost", "Your supplies were intercepted before reaching their destination."),
        162 => ("Enemy supplies destroyed", "Your army intercepted and destroyed an enemy supply convoy."),
        163 => ("Castle work complete", "Work on the county castle has finished."),
        270 => ("Army wages unpaid", "The treasury could not cover troop wages. Raise funds before more seasons go unpaid."),
        271 => ("Mutiny warning", "Your troops have gone unpaid for over a year. They threaten to mutiny if wages go unpaid again."),
        272 => ("Troops mutiny", "Your troops have deserted after going unpaid for too long."),
        273 => ("Rival troops mutiny", "Scouts report that a rival's soldiers mutinied after going unpaid."),
        report::MSG_ARMY_UNFED => ("Troops go unfed", "This army did not get enough food. Send supplies before hunger takes a toll."),
        report::MSG_ARMY_STARVING => ("Troops are starving", "Hunger is taking a toll on this army. Send supplies if you can still reach it."),
        report::MSG_ARMY_PERISHES => ("Army lost to starvation", "This army has perished after going without sufficient food."),
        287 => ("Unpaid troops desert", "Wages remain unpaid and your armies are losing men."),
        // All county events in l2_kingdom::event::EVENT_KINDS.
        135 => ("Rats in the grain stores", "Rats have eaten part of the county's grain stores."),
        136 => ("Cattle disease", "Disease has killed part of the county herd."),
        137 => ("Wolves attack the herd", "Wolves have killed cattle in the county pastures."),
        138 => ("Plague in the county", "Plague has weakened the county's health and caused additional deaths this season."),
        139 => ("Grain stores discovered", "A cache of grain has been added to the county's stores."),
        140 => ("Poor cattle stock", "The county has lost cattle from its herd."),
        141 => ("Cattle herd grows", "An exceptional season has added cattle to the county's herd."),
        142 => ("Wedding fever", "A wave of weddings has brought more births than usual this season."),
        302 => ("Health improves", "Better eating has improved the county's health."),
        303 => ("New fallow field", "A patch of barren ground is now available as a fallow field."),
        304 => ("Weapons discovered", "A cache of weapons has been added to your armoury."),
        305 => ("Treasury donation", "A donation of 500 crowns has been added to your treasury."),
        306 => ("Treasure discovered", "A find has added 1,000 crowns to your treasury."),
        307 => ("Holy relic discovered", "The discovery of a relic has raised happiness in the county."),
        308 => ("County spirits lift", "News of a witch has lifted the people's spirits and increased county happiness."),
        309 => ("Stone discovered", "The county's discovery has added 100 stone to your stores."),
        310 => ("Locusts ruin a field", "Locusts have destroyed a grain field in this county."),
        311 => ("County health declines", "The people blame a curse for a decline in the county's health."),
        312 => ("Treasury fraud", "Fraud has cost your treasury 500 crowns."),
        313 => ("Weapons embezzled", "Weapons have gone missing from your armoury."),
        314 => ("Prize bull lost", "The herd will not produce calves this season. Normal cattle deaths still occur."),
        315 => ("Tax collectors robbed", "Highwaymen waylaid the tax collectors. This county yields no tax revenue this season."),
        316 => ("Termites in the timber", "Termites have destroyed 300 timber from your stores."),
        317 => ("County spirits fall", "The county has grown less happy after losing its songs."),
        // AI correspondence and compose refusals.
        diplo::INVASION => ("Invasion declared", "A rival army has crossed into this county and is laying claim to it."),
        diplo::GIFT_PLEASED => ("Gift warmly received", "The lord welcomes your gift. Relations have improved."),
        diplo::GIFT_GRUDGING => ("Gift acknowledged", "The lord accepts your gift, though the response is cool."),
        diplo::GIFT_CONTEMPTUOUS => ("Gift taken as an insult", "The lord considers your gift too small. Relations have worsened."),
        diplo::COMPLIMENT_FIRST => ("Compliment welcomed", "Your letter was well received. Relations with this lord have improved."),
        diplo::COMPLIMENT_SECOND => ("Compliment acknowledged", "The lord appreciates the kind words, though less than the first time."),
        diplo::COMPLIMENT_ENOUGH => ("Flattery unwelcome", "Repeated compliments have begun to irritate this lord. Relations have worsened."),
        diplo::INSULT => ("An offended reply", "The lord takes offence at your letter. Relations have worsened."),
        diplo::ALLIANCE_REFUSED => ("Alliance declined", "The lord has turned down your offer of an alliance."),
        diplo::ALLIANCE_ACCEPTED => ("Alliance accepted", "The lord has accepted your offer. Your realms are now allies."),
        diplo::ALLIANCE_OFFER => ("Alliance proposed", "This lord invites you to form an alliance. Will you accept?"),
        diplo::ALLIANCE_ENDED => ("Alliance ended", "This lord has ended the alliance between your realms."),
        diplo::ALLIANCE_BROKEN => ("Alliance broken", "The lord considers your actions a breach of the alliance and has ended it."),
        diplo::HELP_REFUSED => ("Help request declined", "Your ally will not send troops to help defend this county."),
        diplo::HELP_AGREED => ("Help promised", "Your ally has agreed to march to help defend this county without payment."),
        diplo::HELP_PRICED => ("Help offered for a price", "Your ally will march to help defend this county if you pay the price shown."),
        diplo::ATTACK_REFUSED => ("Attack request declined", "Your ally will not launch the requested attack."),
        diplo::ATTACK_AGREED => ("Attack request accepted", "Your ally has agreed to march on the target without payment."),
        diplo::ATTACK_PRICED => ("Attack offered for a price", "Your ally will march on the target if you pay the price shown."),
        diplo::WARNING_FIRST => ("Border warning", "The lord protests your army's presence and asks you to respect the border."),
        diplo::WARNING_SECOND => ("Border warning renewed", "The lord again protests your army's presence. Relations are worsening."),
        diplo::NOTICE_OF_REVENGE => ("Rival vows revenge", "The lord has taken offence at your actions and vows to retaliate."),
        192 => ("Unsolicited advice", "A rival has sent advice about your position in the contest for the crown."),
        193 => ("A rival boasts", "A rival writes to boast about their position in the contest for the crown."),
        diplo::ALLIANCE_AT_WAR => ("Alliance refused at war", "This lord will not accept your alliance while your realms are at war."),
        diplo::ALLIANCE_HAVE_ALLY | diplo::ALREADY_IN_ALLIANCE => ("Lord already allied", "This lord already has an ally and cannot form another alliance."),
        diplo::NO_COUNTY_SELECTED => ("Select a county first", "Choose the county your request concerns before sending the letter."),
        diplo::COUNTY_UNOWNED => ("County is independent", "Choose a county held by a ruler before requesting an attack."),
        diplo::COUNTY_NOT_OURS => ("County is not yours", "You can only request help for a county you govern."),
        diplo::COUNTY_UNTHREATENED => ("County is not under threat", "You can only request help when enemy troops threaten the county."),
        diplo::COUNTY_IS_ALLIED => ("Cannot attack an ally", "Choose a target outside the alliance before sending the request."),
        // Seat letters (SendLetter::run and replies). Free text, price and county are drawn separately.
        245 => ("Gift of gold", "The sender has offered this gift:"),
        246 | 247 => ("Letter received", "The sender did not include a note."),
        248 => ("Alliance proposed", "The sender invites you to form an alliance. Will you accept?"),
        249 => ("Alliance ended", "The sender has ended the alliance between your realms."),
        250 => ("Request for help", "The sender asks you to help defend this county:"),
        251 => ("Request for an attack", "The sender asks you to attack this county:"),
        252 => ("Help request accepted", "Your ally has agreed to help defend this county:"),
        253 => ("Help request declined", "Your ally has declined to help defend this county:"),
        254 => ("Attack request accepted", "Your ally has agreed to attack this county:"),
        255 => ("Attack request declined", "Your ally has declined to attack this county:"),
        256 => ("Alliance accepted", "The sender has accepted your offer of an alliance."),
        257 => ("Alliance declined", "The sender has declined your offer of an alliance."),
        258 => ("Player left the game", "A player has left the multiplayer game and forfeited their lands."),
        259 => ("Connection interrupted", "The multiplayer session could not keep the players in sync."),
        // Timed map toasts are label-only. The regular pointer tooltips use a separate painter.
        228 => ("Castle building paused", "Castle building is switched off."),
        229 => ("Castle building enabled", "Castle building is switched on."),
        230 => ("Forestry paused", "Forestry is switched off."),
        231 => ("Forestry enabled", "Forestry is switched on."),
        232 => ("Iron mining paused", "Iron mining is switched off."),
        233 => ("Iron mining enabled", "Iron mining is switched on."),
        234 => ("Blacksmith paused", "The blacksmith is switched off."),
        235 => ("Blacksmith enabled", "The blacksmith is switched on."),
        236 => ("Quarrying paused", "Quarrying is switched off."),
        237 => ("Quarrying enabled", "Quarrying is switched on."),
        // FAQ 291..=295 and the main tutorial series use existing transcriptions.
        215 if matches!(record.shape(), Shape::Paragraphs(_)) => ("Siege preparation", "Build siege engines before launching an assault on a castle."),
        296 if record.category == category::HELP => ("Game files unavailable", "The requested game files could not be found."),
        _ => match record.shape() {
            Shape::Letter | Shape::DiplomaticLetter => ("Dispatch received", "No further details were included in this dispatch."),
            Shape::CountyNotice | Shape::CountyPortrait | Shape::Event => ("County report", "No further details were included in this county report."),
            Shape::Help | Shape::Paragraphs(_) | Shape::Tip => ("Campaign guidance", "No further guidance was included for this topic."),
            Shape::Ending => ("Campaign report", "No further details were included in this campaign report."),
            _ => ("Campaign notice", "No further details were included in this notice."),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Assets;
    use crate::Game;

    #[test]
    fn every_county_event_and_industry_toast_has_its_own_title() {
        let assets = Assets::placeholder();
        let mut game = Game::new(5);
        let ctx = Ctx { game: &mut game, assets: &assets };
        let mut titles = std::collections::BTreeSet::new();
        let events = l2_kingdom::event::EVENT_KINDS.into_iter().map(|kind| (kind.id(), category::EVENT));
        for (group, category) in events.chain((228..=237).map(|id| (id, category::TIP))) {
            let record = Record { group, category, ..Default::default() };
            let name = title(&ctx, &record);
            assert!(titles.insert(name), "duplicate title: {name}");
            assert!(!name.chars().any(|c| c.is_ascii_digit()), "{name}");
            assert!(!body_text(&ctx, &record).starts_with("No further"), "{name}");
        }
    }

    #[test]
    fn classic_text_takes_precedence_but_whitespace_does_not() {
        let group = group::UNDER_FIFTY;
        let offset = 8 + 4 * (usize::from(group) + 1);
        let mut bytes = vec![0; offset];
        bytes[..8].copy_from_slice(b"Textfile");
        for slot in bytes[12..].chunks_exact_mut(4) {
            slot.copy_from_slice(&(offset as u32).to_le_bytes());
        }
        bytes.extend_from_slice(b"  Classic title  \0  Classic body  \0   \0");
        let mut assets = Assets::placeholder();
        assets.shell.eng = Some(crate::shell::Eng::parse(bytes).unwrap());
        let mut game = Game::new(5);
        let ctx = Ctx { game: &mut game, assets: &assets };
        let record = Record { group, ..Default::default() };
        assert_eq!(title(&ctx, &record), "  Classic title  ");
        assert_eq!(body_text(&ctx, &record), "  Classic body  ");
        assert!(body_text(&ctx, &Record { variant: 1, ..record }).contains("50 troops"));
    }

    #[test]
    fn an_empty_player_letter_explains_the_missing_note() {
        let assets = Assets::placeholder();
        let mut game = Game::new(5);
        game.messages.letters = vec![String::new(), "  ".into(), "  Greetings, cousin.  ".into()];
        let ctx = Ctx { game: &mut game, assets: &assets };
        let record = Record { category: category::DIPLOMACY, group: 0xF6, from: 1, ..Default::default() };
        assert_eq!(body_text(&ctx, &record), "The sender did not include a note.");
        assert_eq!(body_text(&ctx, &Record { from: 2, ..record }), "  Greetings, cousin.  ");
    }

    #[test]
    fn an_unknown_event_gets_no_invented_effect() {
        let assets = Assets::placeholder();
        let mut game = Game::new(5);
        let ctx = Ctx { game: &mut game, assets: &assets };
        let record = Record { category: category::EVENT, group: 900, ..Default::default() };
        assert_eq!(title(&ctx, &record), "County report");
        assert_eq!(body_text(&ctx, &record), "No further details were included in this county report.");
    }
}
