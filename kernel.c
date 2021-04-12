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

int main() 
{
	printk("Hello world!\n\r");
	return 0;
}

void start_kernel(start_info_t * start_info)
{
	HYPERVISOR_set_callbacks((unsigned long)hypervisor_callback, (unsigned long)failsafe_callback, 0);
	HYPERVISOR_update_va_mapping((unsigned long) shared_info, __pte(start_info->shared_info), UVMF_INVLPG);

	console_init(start_info);

	printk("\n\r");
	printk("Initialising...                      \n\r");
	printk("       _             _         _     \n\r");
	printk("   ___| |_ ___ ___ _| |_ _ ___| |_   \n\r");
	printk("  |_ -|  _| .'|  _| . | | |_ -|  _|  \n\r");
	printk("  |___|_| |__,|_| |___|___|___|_|    \n\r");
	printk("  minimal\n\r");
	printk("\n\r");

	main();

	console_flush();

	while(1)
	{
		/* Infinite loop to keep the kernel running */
	}
}

