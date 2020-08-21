target remote :3333

# print demangled symbols by default
set print asm-demangle on

!rm 0.stim
monitor tpiu config internal itm.bin uart off 168000000
monitor itm port 0 on

load
step
