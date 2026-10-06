# ssview — stage 1 of a native Goonstation client

Renders a region of a `.dmm` map to PNG using the repo's own DM sources
(type -> icon/icon_state) and `.dmi` assets. No BYOND, no Wine.

    cargo build --release
    ./target/release/ssview <goonstation-root> <map.dmm> out.png [x0 y0 w h]

Known gaps: no layer/plane handling (underfloor pipes draw over floors), no
smoothing of `/auto` walls/windows, no directions/overlays/animation/lighting,
icon vars set via macros or procs are not resolved. Next stages: DM-side state
exporter (via rust_g), live diff stream, input, tgui webview.
