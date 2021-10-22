create table solana_programs
(
    id                  bigserial constraint solana_programs_pk primary key,
    program_id          varchar(88),
    program_name        text,
    type                varchar(100),
    owner               varchar(88),
    constraint solana_programs_type_uindex
        unique (program_id, type)
);