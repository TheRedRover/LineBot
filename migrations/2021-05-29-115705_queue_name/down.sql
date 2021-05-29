-- This file should undo anything in `up.sql`

alter table queues drop column qname;
