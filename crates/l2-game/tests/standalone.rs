use l2_game::Assets;
use l2_mods::Platform;
use l2_view::scene::Ground;

#[test]
fn an_empty_platform_has_playable_field_and_siege_art() {
    let platform = Platform::builder().build().unwrap();
    let assets = Assets::load_original(&platform.vfs).unwrap();
    let battle = assets.battle.as_ref().expect("bundled tactical art");
    for ground in Ground::ALL {
        assert!(battle.has_ground(ground), "the {ground:?} renderer would fall back to the field tiles");
    }
    assert!(l2_game::castle::sheet(3).is_some(), "a stone siege needs a bundled layout");
    assert!(assets.village.as_ref().is_some_and(|art| art.has_grid()), "the village needs interactive resource regions");
}
