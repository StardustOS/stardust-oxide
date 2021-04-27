/* Copyright (C) 2019, Ward Jaradat
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */

#ifndef _CONSOLE_H_
#define _CONSOLE_H_

#include <stdint.h>
#include <xen/xen.h>
#include <xen/io/console.h>

#if defined(__x86_64__)
#include <hypercall-x86_64.h>
#else
#error "Unsupported architecture"
#endif

int console_init(start_info_t *start);
int console_write(char *message);
void console_flush(void);

#define printk(x) console_write(x)

#endif /* _CONSOLE_H_ */
