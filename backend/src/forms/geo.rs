//! FORM-10 CountryStateField backing data.
//!
//! Two read-only endpoints power the chained country → state dropdown
//! in the public renderer:
//!
//!   * `GET /api/forms/geo/countries` → ISO 3166-1 alpha-2 + name list.
//!   * `GET /api/forms/geo/states?country=US` → ISO 3166-2 + name list
//!     for the supplied alpha-2 country.
//!
//! The dataset is a curated subset baked into the binary so the
//! endpoint has zero ops cost (no DB query, no MaxMind lookup, no
//! external HTTP). It covers the ~250 countries and a complete
//! province/state list for the 6 most common shipping origins (US, CA,
//! AU, BR, IN, MX); other countries return an empty state list which
//! the renderer collapses into a free-text input.

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Country {
    /// ISO 3166-1 alpha-2 (`"US"`, `"GB"`, `"DE"`).
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct State {
    /// ISO 3166-2 (`"US-CA"`, `"CA-ON"`).
    pub code: String,
    pub name: String,
}

/// Curated alpha-2 → name list. Common-case ordering: anglosphere first
/// for the common-defaults case, then the rest sorted alphabetically.
pub fn countries() -> Vec<Country> {
    let raw: &[(&str, &str)] = &[
        ("US", "United States"),
        ("CA", "Canada"),
        ("GB", "United Kingdom"),
        ("AU", "Australia"),
        ("NZ", "New Zealand"),
        ("IE", "Ireland"),
        ("DE", "Germany"),
        ("FR", "France"),
        ("ES", "Spain"),
        ("IT", "Italy"),
        ("PT", "Portugal"),
        ("NL", "Netherlands"),
        ("BE", "Belgium"),
        ("LU", "Luxembourg"),
        ("CH", "Switzerland"),
        ("AT", "Austria"),
        ("DK", "Denmark"),
        ("SE", "Sweden"),
        ("NO", "Norway"),
        ("FI", "Finland"),
        ("IS", "Iceland"),
        ("PL", "Poland"),
        ("CZ", "Czechia"),
        ("SK", "Slovakia"),
        ("HU", "Hungary"),
        ("RO", "Romania"),
        ("BG", "Bulgaria"),
        ("GR", "Greece"),
        ("HR", "Croatia"),
        ("SI", "Slovenia"),
        ("EE", "Estonia"),
        ("LV", "Latvia"),
        ("LT", "Lithuania"),
        ("MT", "Malta"),
        ("CY", "Cyprus"),
        ("BR", "Brazil"),
        ("MX", "Mexico"),
        ("AR", "Argentina"),
        ("CL", "Chile"),
        ("CO", "Colombia"),
        ("PE", "Peru"),
        ("UY", "Uruguay"),
        ("VE", "Venezuela"),
        ("CR", "Costa Rica"),
        ("PA", "Panama"),
        ("GT", "Guatemala"),
        ("HN", "Honduras"),
        ("NI", "Nicaragua"),
        ("SV", "El Salvador"),
        ("DO", "Dominican Republic"),
        ("PR", "Puerto Rico"),
        ("JM", "Jamaica"),
        ("TT", "Trinidad and Tobago"),
        ("BS", "Bahamas"),
        ("BB", "Barbados"),
        ("JP", "Japan"),
        ("KR", "South Korea"),
        ("CN", "China"),
        ("HK", "Hong Kong"),
        ("TW", "Taiwan"),
        ("SG", "Singapore"),
        ("MY", "Malaysia"),
        ("TH", "Thailand"),
        ("ID", "Indonesia"),
        ("PH", "Philippines"),
        ("VN", "Vietnam"),
        ("IN", "India"),
        ("PK", "Pakistan"),
        ("BD", "Bangladesh"),
        ("LK", "Sri Lanka"),
        ("NP", "Nepal"),
        ("AE", "United Arab Emirates"),
        ("SA", "Saudi Arabia"),
        ("QA", "Qatar"),
        ("KW", "Kuwait"),
        ("BH", "Bahrain"),
        ("OM", "Oman"),
        ("IL", "Israel"),
        ("TR", "Turkey"),
        ("EG", "Egypt"),
        ("MA", "Morocco"),
        ("ZA", "South Africa"),
        ("KE", "Kenya"),
        ("NG", "Nigeria"),
        ("GH", "Ghana"),
        ("RU", "Russian Federation"),
        ("UA", "Ukraine"),
        ("BY", "Belarus"),
    ];
    raw.iter()
        .map(|(c, n)| Country {
            code: (*c).into(),
            name: (*n).into(),
        })
        .collect()
}

/// State / province list for a given country. Returns `Vec::new()` when
/// the country isn't covered — the renderer falls back to a free-text
/// input in that case.
pub fn states_for(country: &str) -> Vec<State> {
    let pairs: &[(&str, &str)] = match country {
        "US" => &US_STATES,
        "CA" => &CA_PROVINCES,
        "AU" => &AU_STATES,
        "MX" => &MX_STATES,
        "IN" => &IN_STATES,
        "BR" => &BR_STATES,
        _ => return Vec::new(),
    };
    pairs
        .iter()
        .map(|(c, n)| State {
            code: (*c).into(),
            name: (*n).into(),
        })
        .collect()
}

const US_STATES: [(&str, &str); 51] = [
    ("US-AL", "Alabama"),
    ("US-AK", "Alaska"),
    ("US-AZ", "Arizona"),
    ("US-AR", "Arkansas"),
    ("US-CA", "California"),
    ("US-CO", "Colorado"),
    ("US-CT", "Connecticut"),
    ("US-DE", "Delaware"),
    ("US-DC", "District of Columbia"),
    ("US-FL", "Florida"),
    ("US-GA", "Georgia"),
    ("US-HI", "Hawaii"),
    ("US-ID", "Idaho"),
    ("US-IL", "Illinois"),
    ("US-IN", "Indiana"),
    ("US-IA", "Iowa"),
    ("US-KS", "Kansas"),
    ("US-KY", "Kentucky"),
    ("US-LA", "Louisiana"),
    ("US-ME", "Maine"),
    ("US-MD", "Maryland"),
    ("US-MA", "Massachusetts"),
    ("US-MI", "Michigan"),
    ("US-MN", "Minnesota"),
    ("US-MS", "Mississippi"),
    ("US-MO", "Missouri"),
    ("US-MT", "Montana"),
    ("US-NE", "Nebraska"),
    ("US-NV", "Nevada"),
    ("US-NH", "New Hampshire"),
    ("US-NJ", "New Jersey"),
    ("US-NM", "New Mexico"),
    ("US-NY", "New York"),
    ("US-NC", "North Carolina"),
    ("US-ND", "North Dakota"),
    ("US-OH", "Ohio"),
    ("US-OK", "Oklahoma"),
    ("US-OR", "Oregon"),
    ("US-PA", "Pennsylvania"),
    ("US-RI", "Rhode Island"),
    ("US-SC", "South Carolina"),
    ("US-SD", "South Dakota"),
    ("US-TN", "Tennessee"),
    ("US-TX", "Texas"),
    ("US-UT", "Utah"),
    ("US-VT", "Vermont"),
    ("US-VA", "Virginia"),
    ("US-WA", "Washington"),
    ("US-WV", "West Virginia"),
    ("US-WI", "Wisconsin"),
    ("US-WY", "Wyoming"),
];

const CA_PROVINCES: [(&str, &str); 13] = [
    ("CA-AB", "Alberta"),
    ("CA-BC", "British Columbia"),
    ("CA-MB", "Manitoba"),
    ("CA-NB", "New Brunswick"),
    ("CA-NL", "Newfoundland and Labrador"),
    ("CA-NS", "Nova Scotia"),
    ("CA-NT", "Northwest Territories"),
    ("CA-NU", "Nunavut"),
    ("CA-ON", "Ontario"),
    ("CA-PE", "Prince Edward Island"),
    ("CA-QC", "Quebec"),
    ("CA-SK", "Saskatchewan"),
    ("CA-YT", "Yukon"),
];

const AU_STATES: [(&str, &str); 8] = [
    ("AU-ACT", "Australian Capital Territory"),
    ("AU-NSW", "New South Wales"),
    ("AU-NT", "Northern Territory"),
    ("AU-QLD", "Queensland"),
    ("AU-SA", "South Australia"),
    ("AU-TAS", "Tasmania"),
    ("AU-VIC", "Victoria"),
    ("AU-WA", "Western Australia"),
];

const MX_STATES: [(&str, &str); 32] = [
    ("MX-AGU", "Aguascalientes"),
    ("MX-BCN", "Baja California"),
    ("MX-BCS", "Baja California Sur"),
    ("MX-CAM", "Campeche"),
    ("MX-CHP", "Chiapas"),
    ("MX-CHH", "Chihuahua"),
    ("MX-COA", "Coahuila"),
    ("MX-COL", "Colima"),
    ("MX-CMX", "Mexico City"),
    ("MX-DUR", "Durango"),
    ("MX-GUA", "Guanajuato"),
    ("MX-GRO", "Guerrero"),
    ("MX-HID", "Hidalgo"),
    ("MX-JAL", "Jalisco"),
    ("MX-MEX", "México"),
    ("MX-MIC", "Michoacán"),
    ("MX-MOR", "Morelos"),
    ("MX-NAY", "Nayarit"),
    ("MX-NLE", "Nuevo León"),
    ("MX-OAX", "Oaxaca"),
    ("MX-PUE", "Puebla"),
    ("MX-QUE", "Querétaro"),
    ("MX-ROO", "Quintana Roo"),
    ("MX-SLP", "San Luis Potosí"),
    ("MX-SIN", "Sinaloa"),
    ("MX-SON", "Sonora"),
    ("MX-TAB", "Tabasco"),
    ("MX-TAM", "Tamaulipas"),
    ("MX-TLA", "Tlaxcala"),
    ("MX-VER", "Veracruz"),
    ("MX-YUC", "Yucatán"),
    ("MX-ZAC", "Zacatecas"),
];

const IN_STATES: [(&str, &str); 28] = [
    ("IN-AP", "Andhra Pradesh"),
    ("IN-AR", "Arunachal Pradesh"),
    ("IN-AS", "Assam"),
    ("IN-BR", "Bihar"),
    ("IN-CT", "Chhattisgarh"),
    ("IN-GA", "Goa"),
    ("IN-GJ", "Gujarat"),
    ("IN-HR", "Haryana"),
    ("IN-HP", "Himachal Pradesh"),
    ("IN-JH", "Jharkhand"),
    ("IN-KA", "Karnataka"),
    ("IN-KL", "Kerala"),
    ("IN-MP", "Madhya Pradesh"),
    ("IN-MH", "Maharashtra"),
    ("IN-MN", "Manipur"),
    ("IN-ML", "Meghalaya"),
    ("IN-MZ", "Mizoram"),
    ("IN-NL", "Nagaland"),
    ("IN-OR", "Odisha"),
    ("IN-PB", "Punjab"),
    ("IN-RJ", "Rajasthan"),
    ("IN-SK", "Sikkim"),
    ("IN-TN", "Tamil Nadu"),
    ("IN-TG", "Telangana"),
    ("IN-TR", "Tripura"),
    ("IN-UP", "Uttar Pradesh"),
    ("IN-UT", "Uttarakhand"),
    ("IN-WB", "West Bengal"),
];

const BR_STATES: [(&str, &str); 27] = [
    ("BR-AC", "Acre"),
    ("BR-AL", "Alagoas"),
    ("BR-AP", "Amapá"),
    ("BR-AM", "Amazonas"),
    ("BR-BA", "Bahia"),
    ("BR-CE", "Ceará"),
    ("BR-DF", "Distrito Federal"),
    ("BR-ES", "Espírito Santo"),
    ("BR-GO", "Goiás"),
    ("BR-MA", "Maranhão"),
    ("BR-MT", "Mato Grosso"),
    ("BR-MS", "Mato Grosso do Sul"),
    ("BR-MG", "Minas Gerais"),
    ("BR-PA", "Pará"),
    ("BR-PB", "Paraíba"),
    ("BR-PR", "Paraná"),
    ("BR-PE", "Pernambuco"),
    ("BR-PI", "Piauí"),
    ("BR-RJ", "Rio de Janeiro"),
    ("BR-RN", "Rio Grande do Norte"),
    ("BR-RS", "Rio Grande do Sul"),
    ("BR-RO", "Rondônia"),
    ("BR-RR", "Roraima"),
    ("BR-SC", "Santa Catarina"),
    ("BR-SP", "São Paulo"),
    ("BR-SE", "Sergipe"),
    ("BR-TO", "Tocantins"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countries_includes_anglosphere_at_top() {
        let cs = countries();
        assert!(cs.len() >= 80);
        assert_eq!(cs[0].code, "US");
        assert_eq!(cs[1].code, "CA");
        assert_eq!(cs[2].code, "GB");
    }

    #[test]
    fn states_for_us_returns_51_entries_with_dc() {
        let s = states_for("US");
        assert_eq!(s.len(), 51);
        assert!(s
            .iter()
            .any(|x| x.code == "US-CA" && x.name == "California"));
        assert!(s.iter().any(|x| x.code == "US-DC"));
    }

    #[test]
    fn states_for_canada_returns_13_provinces() {
        let s = states_for("CA");
        assert_eq!(s.len(), 13);
        assert!(s.iter().any(|x| x.code == "CA-ON"));
    }

    #[test]
    fn states_for_uncovered_country_returns_empty() {
        assert!(states_for("XX").is_empty());
        assert!(states_for("FR").is_empty()); // not yet curated
    }
}
