use super::*;
use trickcal_shooting_balddagu::chat::ChatChannel;

#[test]
fn all_korean_clips_decode_without_trimming_and_cover_every_hero() {
    let mut long = 0;
    assert_eq!(bank::CLIPS.len(), 309);
    for hero in 0..15 {
        for cue in [
            Cue::Select,
            Cue::Spawn,
            Cue::Start,
            Cue::Win,
            Cue::Lose,
            Cue::Death,
            Cue::Cheek,
            Cue::Line,
            Cue::Skill,
            Cue::Ultimate,
        ] {
            assert!(
                bank::CLIPS.iter().any(|c| c.hero == hero && c.cue == cue),
                "{hero} {cue:?}"
            );
        }
    }
    for clip in bank::CLIPS {
        assert!(!clip.speaker.is_empty() && !clip.text.trim().is_empty(), "{}", clip.name);
        let original =
            SoundBuffer::raw_generic(DataSource::Memory(Cursor::new(clip.bytes.to_vec()))).unwrap();
        let buffer = decode(clip.bytes).unwrap_or_else(|error| panic!("{}: {error}", clip.name));
        let data = buffer.data_ref();
        assert_eq!(data.channel_count(), 1);
        assert_eq!(data.sample_rate(), original.sample_rate());
        assert_eq!(
            data.samples().len(),
            original.samples().len() / original.channel_count()
        );
        assert!(data
            .samples()
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 0.801));
        long += usize::from(data.duration().as_secs_f32() > 2.0);
    }
    assert!(long > 30, "Performances must keep their full length");
}

#[test]
fn successful_cast_cues_interrupt_chatter_and_death_interrupts_casts() {
    assert_eq!(Cue::from_sound(SoundCue::Skill(2)), Some(Cue::Skill));
    assert_eq!(Cue::from_sound(SoundCue::Ultimate(2)), Some(Cue::Ultimate));
    assert_eq!(Cue::from_sound(SoundCue::Ready), None);
    let mut audio = Audio::default();
    let mut graph = Graph::new();
    audio.play(&mut graph, 2, Cue::Line, 0, None, V3::default(), 0.75);
    for cue in [Cue::Skill, Cue::Ultimate, Cue::Death] {
        let previous = audio.playing[0].node;
        audio.play(&mut graph, 2, cue, 0, None, V3::default(), 0.75);
        assert!(!graph.is_valid_handle(previous));
        assert_eq!(audio.playing[0].cue, cue);
    }
    let death = audio.playing[0].node;
    audio.play(&mut graph, 2, Cue::Ultimate, 0, None, V3::default(), 0.75);
    assert_eq!(audio.playing[0].node, death);
    // Two players using the same hero retain separate sources and positions.
    for speaker in [1, 2] {
        audio.play(&mut graph, 2, Cue::Skill, speaker,
                   Some(V3::new(speaker as f32, 0.0, 0.0)), V3::default(), 0.75);
    }
    assert_eq!(audio.playing.len(), 3);
    assert_eq!(audio.playing.iter().filter(|p| p.spatial).count(), 2);
}

#[test]
fn voice_priority_overlap_distance_focus_mute_and_cache_are_bounded() {
    let mut audio = Audio::default();
    let mut graph = Graph::new();
    audio.play(&mut graph, 2, Cue::Death, 0, None, V3::default(), 0.75);
    let death = audio.playing[0].node;
    audio.play(&mut graph, 2, Cue::Line, 0, None, V3::default(), 0.75);
    assert_eq!(audio.playing[0].node, death);
    audio.play(&mut graph, 2, Cue::Win, 0, None, V3::default(), 0.75);
    assert!(!graph.is_valid_handle(death));
    for speaker in 1..PLAYERS {
        audio.play(
            &mut graph,
            speaker as u8,
            Cue::Line,
            speaker,
            Some(V3::new(2.0, 0.0, 0.0)),
            V3::default(),
            0.75,
        );
    }
    assert_eq!(audio.playing.len(), 3);
    audio.maintain(&mut graph, 0.0);
    assert!(audio.playing.is_empty());
    audio.play(
        &mut graph,
        2,
        Cue::Spawn,
        0,
        Some(V3::new(40.0, 0.0, 0.0)),
        V3::default(),
        0.75,
    );
    assert!(audio.playing.is_empty());
    audio.focus(false);
    audio.play(&mut graph, 2, Cue::Spawn, 0, None, V3::default(), 0.75);
    assert!(audio.playing.is_empty());
    for index in 0..40 {
        assert!(audio.buffer(index).is_some());
    }
    assert_eq!(audio.cache.len(), 32);
}

#[test]
fn replayed_typed_and_muted_chat_never_triggers_a_character_line() {
    let mut history = ChatHistory::default();
    let mut seen = None;
    let message = |id, voice_line| ChatMessage {
        id,
        sender: 0,
        character: 2,
        channel: ChatChannel::Team,
        text: "음성 대사".into(),
        voice_line,
    };
    history.insert(message(1, true));
    assert!(new_lines(&history, &mut seen, true).is_empty()); // Joining history.
    history.insert(message(2, false));
    assert!(new_lines(&history, &mut seen, true).is_empty()); // User-typed text.
    history.insert(message(3, true));
    assert_eq!(new_lines(&history, &mut seen, true)[0].id, 3);
    history.insert(message(3, true));
    assert!(new_lines(&history, &mut seen, true).is_empty()); // UDP retry.
    history.insert(message(4, true));
    assert!(new_lines(&history, &mut seen, false).is_empty());
    assert!(new_lines(&history, &mut seen, true).is_empty()); // Unmute.
    history.insert(message(6, true));
    assert_eq!(new_lines(&history, &mut seen, true)[0].id, 6);
    history.insert(message(5, true));
    assert!(new_lines(&history, &mut seen, true).is_empty()); // Delayed older cue.
}

#[test]
fn selection_variants_rotate_and_chatter_is_rate_limited() {
    let mut audio = Audio::default();
    let mut graph = Graph::new();
    audio.play(&mut graph, 0, Cue::Select, 0, None, V3::default(), 0.75);
    let name = graph[audio.playing[0].node].name().to_owned();
    audio.play(&mut graph, 0, Cue::Select, 0, None, V3::default(), 0.75);
    assert_eq!(audio.playing.len(), 1);
    assert_ne!(graph[audio.playing[0].node].name(), name);
    audio.stop(&mut graph);
    audio.play(&mut graph, 0, Cue::Line, 0, None, V3::default(), 0.75);
    let node = audio.playing[0].node;
    audio.play(&mut graph, 0, Cue::Line, 0, None, V3::default(), 0.75);
    assert_eq!(audio.playing[0].node, node);
    audio.focus(false);
    audio.maintain(&mut graph, 0.75);
    assert!(audio.playing.is_empty());
}
