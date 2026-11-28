//! Built-in English copy, keyed by the layouts that consume it. No stock
//! language table is needed; a mod can replace any caption with `L2.eng`.

pub(super) fn caption(group: usize, i: usize) -> &'static str {
    fn at(row: &[&'static str], i: usize) -> &'static str {
        row.get(i).copied().unwrap_or("")
    }
    match group {
        4 => at(&["End season"], i),
        5 => at(&["The wool convoy", "The north road merchant", "The royal factor", "The river merchant", "The salt road merchant"], i),
        6 => l2_kingdom::trade::Good::from_id(i).map_or("", l2_kingdom::trade::Good::name),
        7 => at(&["The Steward", "The Chancellor", "The Marshal", "The Castellan", "The Keeper"], i),
        8 => at(NOUNS, i),
        11 => at(FRONT, i),
        16 => at(&["No company", "Vale levies", "Fen archers", "Coastal company", "Border company", "Hill pikes", "Estuary guard", "Ridge riders", "West march", "North watch", "Harbour free lances", "Stoneward company", "Alder veterans"], i),
        17 => at(&["Divide your army", "Detach the chosen troops"], i),
        18 => at(&["Yes", "No", "Cancel"], i),
        19 => at(&["On", "Off"], i),
        20 => at(&["Famine", "Hungry", "Fair", "Well fed", "Thriving"], i),
        21 => at(&["None", "Half", "Normal", "Extra", "Double", "Triple"], i),
        29 => at(&["No season", "Spring", "Summer", "Autumn", "Winter"], i),
        30 => tile(i),
        31 => unit(i),
        32 => at(&["Paused"], i),
        33 => at(&["Send supplies", "Deliver to", "Available", "Send", "Select a county", "Select an army", "No destination"], i),
        34 => at(&["Year"], i),
        35 => at(&["Population", "Prosperity", "Treasury", "Land", "Forces", "Castles", "Overall", "Undecided"], i),
        36 => at(&["The banner stands victorious", "Your armies have prevailed in", "The next charter awaits", "Continue the conquest", "The charter is lost", "The counties have fallen", "Your enemies claim the field", "Return to the gateway", "The conquest is complete", "The roads are open again.", "Your banners stand in every county.", "The harvest may go unguarded.", "A new steward takes up the ledger.", "The watch fires are extinguished.", "Let the heralds carry word.", "The campaign is won."], i),
        37 => at(&["Battle ratings", "Muster", "Lost", "Defeated", "Score"], i),
        38 => at(&["The hall of banners", "Name", "Wins", "Losses", "Score", "Rank"], i),
        39 => at(&["Choose your campaign", "Full conquest", "Skirmish", "", "Crown and Coast", "March of the Vale"], i),
        40 => at(&["Saved conquests", "Save a conquest", "Opening the ledger...", "Writing the ledger...", "The ledger could not be opened.", "Load a conquest", "Import a skirmish", "No entries", "Right-click to return"], i),
        61 => at(&["Tax", "Rations"], i),
        68 => at(&["Inspect a ware", "We receive", "Trade in", "Buy", "Sell", "Buying", "Selling", "Complete the trade", "Total", "Price to buy", "Price to sell", "and", "In your stores", "My lord", "Confirm purchase", "Confirm sale", "The stall has no more.", "Your purse is short.", "There is nothing to sell.", "Choose the quantity to trade."], i),
        69 => at(&["crowns to sign on", "crowns in wages", "Hire this company?", "The treasury cannot pay them.", "No free companies are waiting.", "Unassigned", "Create", "Change", "Cancel", "Continue", "Contentment will fall", "to", "Contentment remains", "at", "Stock of arms", "The treasury holds", "Calling up the people of"], i),
        70 => at(&["Treasury", "Armoury", "Iron", "Stone", "Timber", "Court of", "Standings", "Wages due", "Expected tax"], i),
        71 => at(&["Castle charter", "Timber stockade", "Motte and bailey", "Stone keep", "Curtain castle", "Crown citadel", "Stone required", "Timber required", "Construction takes", "to finish", "", "Quarters for", "troops", "are stationed here.", "View the garrison", "Begin building", "Tax yield bonus", "Seasons", "Still needed", "An enemy garrison holds this castle."], i),
        72 => at(&["Send a gift", "Write a compliment", "Send a rebuke", "Propose an alliance", "End an alliance", "Ask for help", "Request a county", "Relations", "Allied", "At war", "Gift for", "Compliment to", "Rebuke for", "Alliance offer to", "End alliance with", "Request help from", "Request land from", "Send", "Gift amount", "Select a county to defend", "Select a county to request", "Defend", "Request", "Last gift"], i),
        73 => at(&["Population of", "Previous season", "Births", "Deaths", "Recruits", "Departures to", "New arrivals", "This season", "Recorded peak", "people", "No departures.", "No arrivals."], i),
        74 => at(&["Unassigned", "Grain fields", "Cattle pastures", "Dairy work", "Reclamation", "Iron mining", "Stone quarry", "Woodcutting", "Castle works", "Forge"], i),
        75 => at(&["Choose an item on the workbench to change production."], i),
        80 => at(&["The enemy is in sight", "Choose whether to command your forces on the field.", "The enemy advances", "You may fight in person or let the officers decide.", "The walls are under threat", "Choose whether to command the defence yourself.", "Siege lines are drawn", "Take command of the assault or delegate it to your captains."], i),
        81 => match i { 0 => "The field is decided", 8 => "The siege is decided", _ => "" },
        82 => at(&["Victory", "Your banner holds the field.", "Defeat", "Your companies have left the field.", "Retreated", "Your force has withdrawn.", "Castle lost", "The garrison has surrendered.", "Castle taken", "The enemy yields the walls."], i),
        83 => at(&["Preparing the siege", "Catapults", "Siege towers", "Battering rams", "The work will take", "to complete", "Lift siege", "Attack", "None ordered"], i),
        85 => at(&["Contentment in", "Previous season", "Taxes", "Rations", "Health", "Recruitment", "Ale", "This season", "Average contentment", "Other events"], i),
        86 => at(&["Tax in", "Tax rate", "Per household", "This county", "Other counties"], i),
        87 => at(&["Rations", "Ordered", "Served", "Health", "Consumed", "Fed", "", "", "soldiers foraging in the county."], i),
        94..=98 => at(&["First Company", "The Long Watch", "Border Guard", "Crown Levy", "North March", "The Lanterns", "Coast Guard", "East Vanguard", "The Pikes", "Ridge Guard", "The Iron Guard", "West Reserve", "The River Ward", "The Oak Company", "Dawn Company", "Stone Company", "South Watch", "The Riders", "Hearth Guard", "The Falcons", "Tor Guard", "The Spears", "Hill Reserve", "The Beacons"], i),
        99 => at(&["Independent forces"], i),
        100 => if let Some(county) = (i % 20).checked_sub(1) {
            crate::original::COUNTY_NAMES.get(i / 20).map_or("", |names| at(names, county))
        } else { "" },
        101 => at(crate::original::MAP_NAMES, i),
        102 => at(&["Advanced farming", "Exploration", "Nobles", "Armies eat", "Difficulty", "Starting army", "Starting castle", "Starting weapons", "Starting crowns", "County resources", "Turn time", "Battle control"], i),
        103 => at(SETUP_VALUES, i),
        226 => at(&["I commend your stewardship and wish your people a fair harvest.", "Your officers have been careless. The next insult will not pass unanswered.", "Let our houses stand together while the roads between us are still open.", "Our pact has served its purpose. Let us now keep to our own lands."], i),
        _ => "",
    }
}

const FRONT: &[&str] = &[
    "Open Lords II", "A chronicle of crowns", "Single player", "Multiplayer", "Quit", "The campaign gateway", "Campaign", "Load game", "Custom conquest", "Back",
    "Choose a name and banner", "Continue", "Back", "Begin", "Defaults", "Load",
    "Optional media unavailable", "This feature belongs to the classic edition.", "The built-in campaign does not require a disc.", "Skirmish",
    "Sieges", "Field battles", "Grand battles", "Defender", "Attacker", "", "", "men", "Strength", "", "", "", "", "man", "", "Import a map",
    "Fight", "Back", "Armies", "", "Uneven muster", "Even muster", "Recruit", "Light", "Standard", "Veteran", "Elite", "About", "Return", "Select anywhere to return to the title.",
];

const SETUP_VALUES: &[&str] = &[
    "Simple", "Detailed", "Visible", "Hidden", "One", "Two", "Three", "Four", "Five", "Off", "On",
    "Easy", "Standard", "Hard", "Expert", "None", "Small", "Medium", "Large", "None", "Stockade", "Motte", "Keep", "Curtain", "Citadel",
    "None", "Small", "Medium", "Large", "100", "500", "1000", "2500", "5000", "Scarce", "Balanced", "Abundant",
    "30 sec", "1 min", "2 min", "4 min", "8 min", "10 min", "Unlimited", "Humans only", "All sides",
];

const NOUNS: &[&str] = &[
    "crown", "crowns", "sack", "sacks", "head", "head", "bushel", "bushels", "person", "people", "plot", "plots",
    "tonne of iron", "tonnes of iron", "tonne of stone", "tonnes of stone", "tonne of timber", "tonnes of timber",
    "pike", "pikes", "bow", "bows", "mace", "maces", "crossbow", "crossbows", "sword", "swords", "mail", "mail", "worker", "workers",
    "farmer", "farmers", "herder", "herders", "dairy hand", "dairy hands", "builder", "builders", "miner", "miners", "quarrier", "quarriers", "woodcutter", "woodcutters", "mason", "masons", "smith", "smiths", "soldier", "soldiers",
    "peasant", "peasants", "crossbowman", "crossbowmen", "maceman", "macemen", "swordsman", "swordsmen", "pikeman", "pikemen", "archer", "archers", "knight", "knights", "season", "seasons", "engine", "engines", "turn", "turns", "man total", "men total",
];

fn tile(i: usize) -> &'static str {
    match i {
        0 => "Open ground", 1 => "Road", 3 => "Water", 4 => "High ground", 5 => "Woodland",
        7 => "County seat", 8 => "Castle", 9 => "Quarry", 10 => "Timber stand", 11 => "Iron workings",
        14 => "Castle works", 15 => "Castle repairs", 22 => "Scrub separates the worked fields.",
        23 => "A road links settlements and carries trade.", 24 => "This waterway blocks movement on foot.",
        25 => "Steep rock makes travel difficult.", 26 => "These trees can be harvested for timber.",
        27 => "The county seat gathers the local stores and levies.", 48 => "Village", 49 => "A working village and its fields.",
        50 => "Ruined village", 51 => "The houses stand empty.", 53 => "Stone can be quarried here.", 59 => "A free company is waiting for hire.",
        61 => "Woodcutters bring in timber from this stand.", 69 => "The workings yield iron ore.", 77 => "Not operating", 78 => "Producing", 82 => "The forge", 83 => "The smiths fashion weapons from the stores.",
        _ => "",
    }
}

fn unit(i: usize) -> &'static str {
    match i {
        0 => "Merchant", 2 => "Supply train", 5 => "People on the road", 6 => "Army", 8 => "Wages", 9 => "Raised in",
        12 => "This merchant buys and sells supplies and arms.", 13 => "These people are travelling to a new home.",
        14 => "This train carries goods to a destination.", 15 => "A rival banner commands these troops.",
        16 => "This company marches under your banner.", 20 => "Mustered", 22 => "Movement left", 23 => "Supplies exhausted",
        24 => "Supplies are scarce", 25 => "Supplies are steady", 26 => "Stores are full", 27 => "Starving", 28 => "Hungry", 29 => "Fit", 30 => "Well fed", _ => "",
    }
}
