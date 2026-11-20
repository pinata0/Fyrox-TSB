use super::*;

#[test]
fn screen_routing_respects_visible_menu_priority() {
    for selecting in [false, true] {
        assert_eq!(Track::screen(true, false, selecting), Track::Main);
        assert_eq!(Track::screen(false, true, selecting), Track::Lobby);
        assert_eq!(Track::screen(true, true, selecting), Track::Main);
    }
    assert_eq!(Track::screen(false, false, true), Track::Selection);
    assert_eq!(Track::screen(false, false, false), Track::Battle);
}

#[test]
fn four_complete_stereo_tracks_decode_and_stream_in_small_chunks() {
    for (track, seconds) in [
        (Track::Main, 135.0),
        (Track::Lobby, 91.0),
        (Track::Selection, 165.0),
        (Track::Battle, 126.0),
    ] {
        let (_, bytes) = track.asset();
        let buffer =
            SoundBuffer::raw_generic(DataSource::Memory(Cursor::new(bytes.to_vec()))).unwrap();
        assert_eq!(buffer.channel_count(), 2);
        assert_eq!(buffer.sample_rate(), 44100);
        let duration = buffer.samples().len() as f32 / 88200.0;
        assert!((duration - seconds).abs() < 2.0, "{track:?}: {duration}");
        assert!(buffer.samples().iter().all(|s| s.is_finite()));
        assert!(buffer.samples().iter().any(|s| s.abs() > 0.05));
        let streaming =
            SoundBuffer::raw_streaming(DataSource::Memory(Cursor::new(bytes.to_vec()))).unwrap();
        assert!(matches!(streaming, SoundBuffer::Streaming(_)));
        assert!(streaming.samples().len() < 44100 * 2 * 4);
    }
}

#[test]
fn transitions_keep_one_source_and_same_track_never_restarts() {
    let mut music = Music::default();
    let mut g = Graph::new();
    let settle = |music: &mut Music, g: &mut Graph, track| {
        for _ in 0..50 {
            music.tick(g, track, 0.35, 0.02);
        }
    };
    settle(&mut music, &mut g, Track::Main);
    let old = music.node;
    g[old].cast_mut::<Sound>().unwrap().set_playback_time(12.0);
    settle(&mut music, &mut g, Track::Main);
    assert_eq!(music.node, old);
    assert_eq!(g[old].cast::<Sound>().unwrap().playback_time(), 12.0);
    music.tick(&mut g, Track::Lobby, 0.35, 0.02);
    assert_eq!(music.node, old); // The outgoing song is still fading.
    settle(&mut music, &mut g, Track::Lobby);
    assert!(!g.is_valid_handle(old));
    assert_eq!(music.current, Some(Track::Lobby));
    assert_eq!(
        g.pair_iter()
            .filter(|(_, n)| n.cast::<Sound>().is_some())
            .count(),
        1
    );
    // A rapid reversal while fading keeps the existing source and playhead.
    let lobby = music.node;
    music.tick(&mut g, Track::Selection, 0.35, 0.02);
    settle(&mut music, &mut g, Track::Lobby);
    assert_eq!(music.node, lobby);
    for track in [Track::Selection, Track::Battle, Track::Main] {
        settle(&mut music, &mut g, track);
        assert_eq!(music.current, Some(track));
        let sound = g[music.node].cast::<Sound>().unwrap();
        assert!(sound.is_looping());
        assert_eq!(sound.spatial_blend(), 0.0);
        assert!((sound.gain() - 0.35).abs() < 0.001);
        assert_eq!(
            g.pair_iter()
                .filter(|(_, n)| n.cast::<Sound>().is_some())
                .count(),
            1
        );
    }
}

#[test]
fn mute_focus_and_scene_replacement_preserve_music_without_touching_sfx() {
    let mut music = Music::default();
    let mut g = Graph::new();
    let sfx: Handle<Node> = SoundBuilder::new(BaseBuilder::new().with_name("SFX"))
        .with_status(Status::Playing)
        .with_gain(0.65)
        .build(&mut g)
        .to_base();
    music.tick(&mut g, Track::Battle, 0.35, 0.1);
    let old = music.node;
    g[old].cast_mut::<Sound>().unwrap().set_playback_time(9.5);
    music.tick(&mut g, Track::Battle, 0.0, 0.1);
    assert_eq!(g[old].cast::<Sound>().unwrap().gain(), 0.0);
    assert_eq!(g[old].cast::<Sound>().unwrap().status(), Status::Paused);
    music.focus(false);
    music.tick(&mut g, Track::Battle, 0.7, 0.1);
    assert_eq!(g[old].cast::<Sound>().unwrap().status(), Status::Paused);
    music.focus(true);
    music.tick(&mut g, Track::Battle, 0.7, 0.1);
    assert_eq!(music.node, old);
    assert_eq!(g[old].cast::<Sound>().unwrap().playback_time(), 9.5);
    assert_eq!(g[old].cast::<Sound>().unwrap().status(), Status::Playing);
    music.detach(&mut g);
    assert!(!g.is_valid_handle(old));
    assert_eq!(g[sfx].cast::<Sound>().unwrap().gain(), 0.65);
    let mut next_scene = Graph::new();
    music.tick(&mut next_scene, Track::Battle, 0.35, 0.1);
    assert_eq!(
        next_scene[music.node]
            .cast::<Sound>()
            .unwrap()
            .playback_time(),
        9.5
    );
}
