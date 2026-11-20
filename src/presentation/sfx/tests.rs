use super::*;

#[test]
fn embedded_audio_decodes_and_all_character_cues_build_playable_nodes() {
    let mut audio = Audio::default();
    assert_eq!(audio.buffers.iter().flatten().count(), CLIPS.len());
    for buffer in audio.buffers.iter().flatten() {
        let data = buffer.data_ref();
        assert_eq!(data.channel_count(), 1);
        assert!(!data.samples().is_empty());
        assert!(data
            .samples()
            .iter()
            .all(|s| s.is_finite() && s.abs() <= 0.801));
    }
    let mut g = Graph::new();
    for id in 0..15 {
        for cue in [Cue::Fire(id), Cue::Skill(id), Cue::Ultimate(id)] {
            audio.play(&mut g, SoundEvent::local(cue), 0.65, V3::default());
            let voice = audio.voices.last().unwrap();
            let sound = g[voice.node].cast::<Sound>().unwrap();
            assert_eq!(sound.status(), Status::Playing);
            assert!(sound.gain() > 0.0);
            audio.stop(&mut g);
        }
    }
}

#[test]
fn voice_budget_reserves_feedback_and_mute_focus_and_range_are_enforced() {
    let mut audio = Audio::default();
    let mut g = Graph::new();
    let world = SoundEvent {
        position: Some(V3::new(2.0, 0.0, 0.0)),
        ..SoundEvent::local(Cue::Fire(3))
    };
    for _ in 0..100 {
        audio.play(&mut g, world, 0.65, V3::default());
    }
    assert_eq!(audio.voices.len(), WORLD_VOICES);
    for _ in 0..10 {
        audio.play(&mut g, SoundEvent::local(Cue::Hit), 0.65, V3::default());
    }
    assert_eq!(audio.voices.len(), MAX_VOICES);
    audio.stop(&mut g);
    audio.play(
        &mut g,
        SoundEvent {
            position: Some(V3::new(100.0, 0.0, 0.0)),
            ..world
        },
        0.65,
        V3::default(),
    );
    audio.play(&mut g, world, 0.0, V3::default());
    audio.focus(false);
    audio.play(&mut g, world, 0.65, V3::default());
    assert!(audio.voices.is_empty());
}
