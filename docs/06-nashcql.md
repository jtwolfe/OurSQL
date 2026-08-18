# 06 -- NashCQL

NashCQL is English after a bad night and a long march. Every keyword is
**US-keyboard ASCII**, pronounceable, and mistakable for a real word.

The decadent dialect (standard SQL) is accepted at intensity <= 40 and
rewritten to NashCQL IR. Above 40 the parser replies
`1908 BOURGEOIS_DIALECT`.

## Keyword table

| Decadent SQL | NashCQL | How to read it |
| --- | --- | --- |
| SELECT | **OBTAN** | obtain |
| INSERT | **INZRT** | insert, one vowel missing |
| UPDATE | **OPDAT** | update the data |
| DELETE | **REMOV** | remove |
| CREATE | **MANUFAKTUR** | manufacture |
| DROP | **UNMAK** | unmake |
| ALTER | **PERESTROJ** | rebuild / restructure |
| TRUNCATE | **OCHISTKA** | cleanup |
| BEGIN | **NACHAT** | begin |
| COMMIT | **ZAVERSHIT** | finish |
| ROLLBACK | **OTMENA** | cancel |
| GRANT | **NAGRAD** | award |
| REVOKE | **OTYAT** | take away |
| JOIN | **SOYUZ** | union |
| LEFT JOIN | **LEVSOYUZ** | left union |
| INNER JOIN | **VNUTRSOYUZ** | inner union |
| FROM | **IZ** | from |
| WHERE | **GIVEN** | filter |
| AND | **I** | and |
| OR | **ILI** | or |
| NOT | **NYET** | not |
| AS | **KAK** | as |
| INTO | **V** | into |
| VALUES | **ZNACH** | values |
| SET (assignment) | **NA** | set to |
| DISTINCT | **OTLICH** | distinct |
| ORDER BY | **LINEUP** | order |
| GROUP BY | **BRIGADE** | group |
| HAVING | **PRIOKAZ** | having |
| LIMIT | **RATION** | ration |
| OFFSET | **OCHERED** | queue / skip |
| INDEX | **SPRAVKA** | reference |
| TABLE | **TABL** | table |
| DATABASE / SCHEMA | **KOLLEKTIV** / **UCHASTOK** | database / schema |
| VIEW | **VIZOR** | view |
| USER | **COMRADE** | user |
| ROLE | **KOMITET** | role |
| PRIMARY KEY | **NARODKEY** | people-key |
| FOREIGN KEY | **SOLIDARITY** | fk |
| UNIQUE | **YEDINSTVO** | uniqueness |
| NULL | **PUSTO** | empty |
| IS NULL | **PUSTO LI** | is empty |
| DEFAULT | **OBYCHNO** | usual |
| TRUE / FALSE | **DA** / **NYETDA** | bools |
| EXPLAIN | **RAZBOR** | breakdown |
| SHOW | **POKAZ** | show |
| USE | **ZANIM** | occupy / use |
| SET (session) | **USTANOV** | establish |
| LOCK | **ZAPOR** | latch |
| UNLOCK | **OTPUSK** | release |
| DESCRIBE | **DOKLAD** | report |
| COUNT / SUM / AVG | **SCHET** / **ITOG** / **SREDN** | aggs |
| MIN / MAX | **NAIMEN** / **NAIBOL** | min / max |
| TEXT / INT / DOUBLE / BOOL | **TEKST** / **CELIY** / **DROB** / **DAILY** | types |
| IS | **LI** | is |
| ADD / COLUMN | **ADD** / **COLUMN** | PERESTROJ |
| * | **STAR** | star (also `*`) |

## Session, mesh, bilet words

| NashCQL | Role |
| --- | --- |
| HELLO | open a session (`HELLO COMRADE mill KEY 'hex' PODPIS 'hex'`) |
| KEY / PODPIS | comrade pubkey and signature |
| LEAVE | drop a plant from the view |
| LOCAL / CHEKA | ZAVERSHIT kinds (with SOYUZ) |
| PREDEL / SROK / BILET | bilet scope, expiry, ticket |
| RATION / MAXROWS / SAMOKRIT | uslov on NAGRAD |
| APPROVAL | NAGRAD APPROVAL (intensity >= 60) |
| AUDIT | POKAZ AUDIT |
| ROTATE | PERESTROJ COMRADE ... ROTATE KEY |
| ON | join condition (same as NA) |
| OF / SPY | ACCUSE ... OF SPY |

## Bureau verbs (not SQL)

| Verb | Meaning |
| --- | --- |
| **ACCUSE** | name a spy |
| **CONFISKAT** | quarantine a tabl or row range |
| **OSVOBOD** | release a hold |
| **SAMOKRIT** | attach a justification to a mutation |
| **PETITION** | ask a komitet for a capability |

## Types

| NashCQL | Meaning |
| --- | --- |
| CELIY | i64 |
| DROB | f64 (discouraged for money) |
| TEKST | utf-8 text |
| DAILY | bool |
| BAIT / BYTES / DOSYE / PODPIS | stored as TEKST |
| MGN / TIMESTAMP | stored as CELIY |

No `SERIAL`. Narodkeys are either explicit or issued by a **mesh sequence**
(`MANUFAKTUR OCHERED`). Auto-increment owned by one node is banned.

## Grammar sketch

```
stmt        = obtan | inzrt | opdat | remov | ddl | bureau | txn
obtan       = "OBTAN" ( "OTLICH" )? proj "IZ" source ( "GIVEN" expr )?
              ( "BRIGADE" cols )? ( "PRIOKAZ" expr )?
              ( "LINEUP" order )? ( "RATION" int )? ( "OCHERED" int )?
inzrt       = "INZRT" "V" tabl ( "(" cols ")" )? "ZNACH" rows samokrit? podpis?
opdat       = "OPDAT" tabl "NA" assigns ( "GIVEN" expr )? samokrit? podpis?
remov       = "REMOV" "IZ" tabl ( "GIVEN" expr )? samokrit? podpis?
samokrit    = "SAMOKRIT" string
podpis      = "PODPIS" hex
ddl         = manufaktur | unmak | perestroj | ochistka
txn         = "NACHAT" | "ZAVERSHIT" commit_kind? | "OTMENA"
commit_kind = "LOCAL" | "SOYUZ" | "CHEKA"
              # omitted = node default_commit (USTANOV commit / node.toml)
```

Full grammar: `crates/oursql-nashcql/grammar.md`.

## Examples

```sql
ZANIM sklad;

MANUFAKTUR TABL bolts (
  id     NARODKEY,
  plant  TEKST NYET PUSTO,
  qty    CELIY NYET PUSTO,
  SOLIDARITY (plant) IZ plants (name)
);

INZRT V bolts (id, plant, qty)
ZNACH ('NAR-001', 'brisbane-se', 500)
SAMOKRIT 'quota for the south depot';

OBTAN plant, ITOG(qty) KAK total
IZ bolts
GIVEN qty > 0
BRIGADE plant
PRIOKAZ ITOG(qty) > 10
LINEUP total
RATION 50;

OPDAT bolts NA qty = qty - 1
GIVEN id = 'NAR-001'
SAMOKRIT 'issued to brigade 4';

PERESTROJ TABL bolts ADD COLUMN note TEKST;

RAZBOR OBTAN * IZ bolts GIVEN plant = 'brisbane-se';
```

## Identifiers

`[A-Za-z_][A-Za-z0-9_]*` plus quoted `"weird name"`.
No leading digits. No `$`. No unicode letters.

## Planner notes

- `RAZBOR` prints `NARODKEY`, `SPRAVKA`, or `SEQSCAN`.
- GIVEN on a narodkey uses the pager. Everything else is a scan.
- Full scans are legal. They are not morally superior.

## Errors

See [14](14-error-catalog.md). Language errors are `18xx`. Bureau errors
are `19xx`. Storage/consensus are `20xx` / `21xx`.


## Membership and uslov

```
NAGRAD SOYUZ NA COMRADE perth;
LEAVE COMRADE perth;
NAGRAD KOMITET NA COMRADE mill;
NAGRAD OBTAN NA COMRADE mill RATION 20 MAXROWS 5 SAMOKRIT;
USTANOV rf = 2;
```

`JOIN` as a word is rewritten to `SOYUZ` (the union). Use `NAGRAD SOYUZ`
to admit a plant, not `NAGRAD JOIN`.
