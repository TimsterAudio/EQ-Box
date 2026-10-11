# Panel design

EQ Box uses Drone Box's panel language, so the two look like the same family of hardware. Drone Box's `docs/DESIGN.md` is the reference. This page covers how EQ Box applies it.

## Shared with Drone Box

- **Panels:** Black `#171717` with cream legends, or Silver `#d5d2ca` with near-black legends and a faint brushed texture. Section frames are drawn in the legend colour at 30% (black panel) or 42% (silver panel).
- **The stripe** in red, orange, yellow and cream under the top strip.
- **Type:**
  - Space Grotesk for the maker wordmark badge
  - Michroma, slanted −12°, for the logo
  - Barlow Semi Condensed, uppercase and letter-spaced, for legends and section titles
  - Share Tech Mono in red with a glow for the displays and readouts
- **Controls:** knurled black knobs turning through 270°, and faders with black caps and a line across them. Hardware keys are red, orange, yellow, cream or grey, with a red LED above when they hold a state.
- **Every knob and fader has a permanent red readout** under its label, and the LED display shows the last control touched.
- **Knob colour shows what a control does:** green (tone) for Freq and Q, blue for the audio buffer, and black for levels (Preamp). The **Knobs** switch chooses between Colour (a tinted top) and Pointer (a black knob with a coloured pointer).
- **Fit to window:** the panel is laid out at 1640 × 884 and scaled to fit the window, so it never re-wraps.

## EQ Box specifics

| Element | Choice |
|---|---|
| Band gain | **Fader**, like a level, 0 dB at the bold tick |
| Band frequency, Q | **Knob**, green (tone family) |
| Filter type (6 choices) | **Two LED key rows**, orange: LO SH / BELL / HI SH over HPF / NOTCH / LPF |
| Band on/off | **Lit key** with no LED: it glows in the band's colour when on, and is a dark lens when off |
| Bypass | **Yellow key** with LED |
| Run / Stop | **Red key** with LED, like Drone Box's Play / Stop |
| Load / Save presets | **Cream keys**. Flat and Reset are **grey** (utility) keys |
| Panel and Knobs switches | **Grey keys** with LEDs, as in Drone Box |
| Analyser on/off, colour, style | **Tabs on the Response frame's top edge**, at the right (like Drone Box's Window tab). Colour and Style show their setting in display red and step through the choices on each click |
| Response graph | Drawn as a large red display: dark red background, dim red grid, total response in display red with a glow |
| Band colours | Ten colours used for the band's point and curve on the graph and the chip on its strip: red, orange, yellow, lime, green, teal, blue, violet, pink and cream |
| Selected band | Its strip's frame turns orange (the focus colour), and its point gets a cream ring |
| Audio devices | Lists styled as red displays |

## Layout (design units)

- **Top strip, 0–110:** wordmark, logo and model line; the LED display; the APO preset keys; In and Out meters; Preamp; the Panel and Knobs switches; Bypass; Run / Stop.
- **Stripe** at 110.
- **Row 2, 136–500:** **Response** (the graph) and **Audio** (system, input, output, buffer, rescan, dropouts, help).
- **Row 3, 518–848:** ten **Band** strips, each with the two rows of type keys, the gain fader on the left (its readout shows `--` for the types without gain), the Freq and Q knobs on the right, and the On key.
- **Footer:** usage hints, and notices such as preset warnings.
