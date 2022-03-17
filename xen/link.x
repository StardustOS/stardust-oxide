OUTPUT_FORMAT("elf64-x86-64", "elf64-x86-64", "elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)
SECTIONS
{
  . = 0x0;
  _text = .;			/* Text and read-only data */
  .text : {
	*(.text)
	*(.gnu.warning)
	} = 0x9090

  _etext = .;			/* End of text section */

  .rodata : { *(.rodata) *(.rodata.*) }
  . = ALIGN(4096);
  _erodata = .;

  /* newlib initialization functions */
  . = ALIGN(64 / 8);
  PROVIDE (__preinit_array_start = .);
  .preinit_array     : { *(.preinit_array) }
  PROVIDE (__preinit_array_end = .);
  PROVIDE (__init_array_start = .);
  .init_array     : { *(.init_array) }
  PROVIDE (__init_array_end = .);
  PROVIDE (__fini_array_start = .);
  .fini_array     : { *(.fini_array) }
  PROVIDE (__fini_array_end = .);

  .ctors : {
        __CTOR_LIST__ = .;
        *(.ctors)
	CONSTRUCTORS
        QUAD(0)
        __CTOR_END__ = .;
        }

  .dtors : {
        __DTOR_LIST__ = .;
        *(.dtors)
        QUAD(0)
        __DTOR_END__ = .;
        }

  .data : {			/* Data */
	*(.data)
	}

  _edata = .;			/* End of data section */

  __bss_start = .;		/* BSS */
  .bss : {
	*(.bss)
        *(.app.bss)

        . = ALIGN(4096) ;
        __STACK_START = . ;
        . += 65536; /* Defines stack size, must be power of 2 */
        __STACK_END = . ;
  }
  _end = . ;


  /* Sections to be discarded */
  /DISCARD/ : {
  *(.text.exit)
  *(.data.exit)
  *(.exitcall.exit)
  }

  /* Stabs debugging sections.  */
  .stab 0 : { *(.stab) }
  .stabstr 0 : { *(.stabstr) }
  .stab.excl 0 : { *(.stab.excl) }
  .stab.exclstr 0 : { *(.stab.exclstr) }
  .stab.index 0 : { *(.stab.index) }
  .stab.indexstr 0 : { *(.stab.indexstr) }
  .comment 0 : { *(.comment) }

}
