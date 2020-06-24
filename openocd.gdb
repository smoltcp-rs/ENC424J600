target remote :3333

# print demangled symbols by default
set print asm-demangle on

!rm itm.log
monitor tpiu config internal itm.log uart off 168000000
monitor itm port 0 on

load
step
