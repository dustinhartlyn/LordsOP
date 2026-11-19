use l2_formats::hlp::{Line, Span, Topic};

pub(super) fn topic(id: u32) -> Option<Topic> {
    let (title, paragraphs, links): (&str, &[&str], &[(u32, &str)]) = match id {
        0 => ("Field guide", &[
            "Take a banner, keep your people fed, and bring the rival lords to terms. A campaign takes you through a series of provinces. A custom game lets you pick a province and its starting conditions.",
            "Use the minimap to go to a county. Move the pointer against the edge of the map to scroll. Select End Turn to advance the season after issuing your orders.",
            "Choose a topic below for your next steps.",
        ], &[(1, "Counties and supplies"), (2, "Armies and battles"), (3, "Saving and multiplayer")]),
        1 => ("Counties and supplies", &[
            "Select an owned county and use its sidebar to review population, stores, taxes and rations. The farms need workers and land. Grain and cattle both help feed the population.",
            "Move workers between farming and industry, and open the production rows to set priorities. Wood, stone and iron can be traded. The blacksmith turns supplies into weapons.",
            "Use the supplies screen to send food or materials to another county. Check your stocks before winter. High taxes and low rations can make a county revolt.",
        ], &[(0, "Field guide contents"), (2, "Armies and battles")]),
        2 => ("Armies and battles", &[
            "Raise an army from an owned county with the levy button. Raising troops takes workers out of the economy. Equip them in the armoury and watch the cost of keeping them in the field.",
            "Roads let armies travel faster. Water, woods and mountains can block a route. Fortifying a county gives its garrison a place to hold off an attack.",
            "In a tactical battle, protect archers with infantry and use terrain to your advantage. Check Options for the Modern and Classic battle-control schemes.",
        ], &[(0, "Field guide contents"), (1, "Counties and supplies")]),
        3 => ("Saving and multiplayer", &[
            "Use the game menu to save or load. Standalone and classic saves use different folders because their worlds are different.",
            "For multiplayer, choose the multiplayer option at the title screen. The host picks the game and sends the room code to the other players. When all players are ready, the host can start.",
            "Everyone should use the same game build, asset mode and mods.",
        ], &[(0, "Field guide contents")]),
        _ => return None,
    };
    let line = |text: &str, jump| Line { spans: vec![Span { text: text.into(), jump }] };
    let lines = paragraphs.iter().map(|s| line(s, None))
        .chain(links.iter().map(|&(id, text)| line(text, Some(id))))
        .collect();
    Some(Topic { title: title.into(), lines })
}
