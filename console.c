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

#include "include/os.h"
#include "include/console.h"

#include <xen/sched.h>

static evtchn_port_t console_evt;
extern char _text;
struct xencons_interface * console;

int console_init(start_info_t * start)
{
	console = (struct xencons_interface*) ((machine_to_phys_mapping[start->console.domU.mfn] << 12) + ((unsigned long)&_text));
	console_evt = start->console.domU.evtchn;
	return 0;
}

int console_write(char * message)
{
	struct evtchn_send event;
	event.port = console_evt;
	int length = 0;
	while(*message != '\0')
	{
		XENCONS_RING_IDX data;
		do
		{
			data = console->out_prod - console->out_cons;
			HYPERVISOR_event_channel_op(EVTCHNOP_send, &event);
			mb();
		} while (data >= sizeof(console->out));
		int ring_index = MASK_XENCONS_IDX(console->out_prod, console->out);
		console->out[ring_index] = *message;
		wmb();
		console->out_prod++;
		length++;
		message++;
	}
	HYPERVISOR_event_channel_op(EVTCHNOP_send, &event);
	return length;
}

void console_flush()
{
	while(console->out_cons < console->out_prod)
	{
		HYPERVISOR_sched_op(SCHEDOP_yield, 0);
		mb();
	}
}
