
    /// [`turns_crossed`]: a pin dropping from the left onto its frame
    /// inside an entry's span crosses the entry when it lies past it, and
    /// not when it lies on the side it comes from; turns outside the
    /// other leg's span cross nothing (the verifier's twin blocks drawn
    /// TD, `q --> B` and `a1 --> b1`, see [`pins_past_turns`]).
    #[test]
    fn a_pins_turn_inside_another_legs_span_crosses_it_on_one_side_only() {
        // The entry turns left at both ends; the pin turns left once and
        // ends on its port.
        let entry = [(261.0, -1.0), (331.0, -1.0)];
        let pin = [(274.0, -1.0), (301.0, 0.0)];
        let (se, sp) = ((261.0, 331.0), (274.0, 301.0));
        assert_eq!(turns_crossed(entry, se, pin, sp), 1);
        assert_eq!(turns_crossed(pin, sp, entry, se), 0);
        let short = [(100.0, 1.0), (120.0, -1.0)];
        assert_eq!(turns_crossed(short, (100.0, 120.0), pin, sp), 0);
        assert_eq!(turns_crossed(pin, sp, short, (100.0, 120.0)), 0);
    }
