//! NashCQL surface. Keyword table is the contract from docs/06-nashcql.md.

#![deny(unsafe_code)]

/// A reserved word and its decadent SQL cousin.
#[derive(Clone, Copy, Debug)]
pub struct Keyword {
    pub nash: &'static str,
    pub sql: &'static str,
}

/// Official keyword table. ASCII only. US keyboard.
pub const KEYWORDS: &[Keyword] = &[
    Keyword { nash: "OBTAN", sql: "SELECT" },
    Keyword { nash: "INZRT", sql: "INSERT" },
    Keyword { nash: "OPDAT", sql: "UPDATE" },
    Keyword { nash: "REMOV", sql: "DELETE" },
    Keyword { nash: "MANUFAKTUR", sql: "CREATE" },
    Keyword { nash: "UNMAK", sql: "DROP" },
    Keyword { nash: "PERESTROJ", sql: "ALTER" },
    Keyword { nash: "OCHISTKA", sql: "TRUNCATE" },
    Keyword { nash: "NACHAT", sql: "BEGIN" },
    Keyword { nash: "ZAVERSHIT", sql: "COMMIT" },
    Keyword { nash: "OTMENA", sql: "ROLLBACK" },
    Keyword { nash: "NAGRAD", sql: "GRANT" },
    Keyword { nash: "OTYAT", sql: "REVOKE" },
    Keyword { nash: "SOYUZ", sql: "JOIN" },
    Keyword { nash: "IZ", sql: "FROM" },
    Keyword { nash: "GIVEN", sql: "WHERE" },
    Keyword { nash: "TABL", sql: "TABLE" },
    Keyword { nash: "KOLLEKTIV", sql: "DATABASE" },
    Keyword { nash: "COMRADE", sql: "USER" },
    Keyword { nash: "KOMITET", sql: "ROLE" },
    Keyword { nash: "NARODKEY", sql: "PRIMARY KEY" },
    Keyword { nash: "SOLIDARITY", sql: "FOREIGN KEY" },
    Keyword { nash: "RATION", sql: "LIMIT" },
    Keyword { nash: "LINEUP", sql: "ORDER BY" },
    Keyword { nash: "BRIGADE", sql: "GROUP BY" },
    Keyword { nash: "RAZBOR", sql: "EXPLAIN" },
    Keyword { nash: "POKAZ", sql: "SHOW" },
    Keyword { nash: "ZANIM", sql: "USE" },
    Keyword { nash: "SPRAVKA", sql: "INDEX" },
    Keyword { nash: "NYET", sql: "NOT" },
    Keyword { nash: "PUSTO", sql: "NULL" },
];

pub fn nash_for_sql(sql: &str) -> Option<&'static str> {
    let u = sql.to_ascii_uppercase();
    KEYWORDS.iter().find(|k| k.sql == u).map(|k| k.nash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perestroj_is_alter() {
        assert_eq!(nash_for_sql("alter"), Some("PERESTROJ"));
    }

    #[test]
    fn all_ascii() {
        for k in KEYWORDS {
            assert!(k.nash.bytes().all(|b| b.is_ascii_uppercase()));
        }
    }
}
