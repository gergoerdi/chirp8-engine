# CHIRP-8 Engine

A Rust CHIP-8 VM implementation that doesn't use `std`. Other packages build on this
to implement a complete CHIP-8 computer on various platforms:

* CHIRP-8 SDL (https://github.com/gergoerdi/chirp8-sdl) is an SDL-based implementation 
  targeting "normal" computers. Its main use is in testing the engine itself.
  
* CHIRP-8 AVR (https://github.com/gergoerdi/chirp8-avr) targets 8-bit AVR microcontrollers,
  and is intended to use with a simple circuit consisting of a small handful of components
  that are all breadboard-friendly. Read more about it in my blog post:
  https://gergo.erdi.hu/blog/2017-05-12-rust_on_avr__beyond_blinking/
  
* CHIRP-8 C64 (https://github.com/gergoerdi/chirp8-c64) targets the Commodore 64 home computer
  from the '80s, with a MOS 6502 processor. Read more about it in my blog post: 
  https://gergo.erdi.hu/blog/2021-09-18-rust_on_the_mos_6502__beyond_fibonacci/
