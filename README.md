Simple terminal interface for blocking websites via the /etc/hosts file.

Must be run as sudo as it needs to write to the hosts file, plus a backup
file at /etc/hosts.hb.back

Controls
i 		- add a new domain
j/k 	- down, up
J/K 	- goto bottom, top
d 		- delete selected
space - toggle whether or not selected domain is blocked
