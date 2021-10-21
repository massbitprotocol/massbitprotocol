create table solana_programs
(
    program_id          varchar(88) constraint solana_programs_pk primary key,
    program_name        text,
    type                varchar(100),
    owner               varchar(88)
);