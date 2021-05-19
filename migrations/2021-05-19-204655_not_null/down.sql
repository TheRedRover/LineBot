-- This file should undo anything in `up.sql`

alter table queue_element alter column queue_place drop not null;
