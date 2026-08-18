# 14 — Error catalog

Stable `u16` codes. Clients may switch on the number. Messages are ASCII.

## 18xx — language

| Code | Name | Meaning |
| --- | --- | --- |
| 1801 | BAD_TOKEN | lexer
| 1802 | BAD_GRAMMAR | parser
| 1803 | UNKNOWN_IDENT | tabl/col/comrade
| 1804 | TYPE_FIGHT | type error
| 1805 | PUSTO_WHERE_BANNED | NOT NULL violation
| 1806 | NO_NARODKEY | tabl without people-key
| 1807 | BAD_KEYWORD | unknown verb

## 19xx — bureau

| Code | Name | Retry? |
| --- | --- | --- |
| 1901 | BOURGEOIS_KEYWORDS | notice, not fatal at 25 |
| 1902 | COLLECTIVE_PARTIAL | yes, same plan_id |
| 1903 | REVIEW_WAIT | wait retry_after_ms |
| 1904 | NO_APPROVAL | maybe, after NAGRAD |
| 1905 | GULAG | yes, after ration refill |
| 1906 | CONFISKAT | no, until OSVOBOD |
| 1907 | ACCUSED | notice / mild delay |
| 1908 | BOURGEOIS_DIALECT | rewrite in NashCQL |
| 1909 | SAMOKRIT_REQUIRED | add SAMOKRIT
| 1910 | TOO_MANY_ACCUSATIONS | tomorrow
| 1911 | INTENSITY_DENIED | no
| 1912 | LINE_CONFLICT | rewrite, retry

## 20xx — storage

| Code | Name |
| --- | --- |
| 2001 | WAL_IO
| 2002 | PAGE_CHECKSUM
| 2003 | POOL_EXHAUSTED
| 2004 | RECOVERY_FAILED

## 21xx — mesh / node

| Code | Name |
| --- | --- |
| 2101 | NOT_IN_VIEW
| 2102 | BELOW_QUORUM
| 2103 | UNRELIABLE_PEER
| 2104 | JOIN_REFUSED
| 2105 | REPAIR_NEEDED
| 2106 | BAD_HELLO
| 2107 | CAP_EXPIRED
| 2108 | NODE_BUSY
| 2109 | PERESTROJ_WAIT
| 2110 | UNSIGNED_MUTATION

## Client guidance

Treat 1902 and 1905 as **normal** at intensity 25. Backoff on 1905.
Replay 1902 immediately once.
