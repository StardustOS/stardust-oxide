# Copyright (C) 2019, Ward Jaradat
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 2 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License along
# with this program; if not, write to the Free Software Foundation, Inc.,
# 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.

XEN_TARGET_ARCH = x86_64
ARCH_CFLAGS := -m64
ARCH_ASFLAGS := -m64
ARCH_LDFLAGS := -m elf_x86_64 -T loader.lds
ARCH_OBJS := bootstrap.o

CPPFLAGS += -Iinclude
CPPFLAGS += -Iinclude/x86
CPPFLAGS += -DCONFIG_X86_PAE
CPPFLAGS += -D__XEN_INTERFACE_VERSION__=0x00030203 $(ARCH_CPPFLAGS)
LDFLAGS  += -nostdlib -g $(ARCH_LDFLAGS)
CFLAGS   += -Wall -g $(ARCH_CFLAGS) -fno-stack-protector
ASFLAGS  = -D__ASSEMBLY__ $(ARCH_ASFLAGS)

.PHONY: all clean run target/target/debug/libstardust.a

all: $(ARCH_OBJS) kernel.o console.o traps.o target/target/debug/libstardust.a
	$(LD) $(LDFLAGS) $^ -o minimal
	gzip -f -9 -c minimal >minimal.gz

target/target/debug/libstardust.a:
	cargo build
	cbindgen --config cbindgen.toml --crate stardust --output include/libstardust.h

clean:
	rm -f *.o
	rm -f minimal
	rm -f minimal.gz
	cargo clean
