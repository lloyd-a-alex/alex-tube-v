$file = 'src/main.rs'
$content = [System.IO.File]::ReadAllText($file)

# 1. LazyLock
$content = $content.Replace('once_cell::sync::Lazy<', 'std::sync::LazyLock<')
$content = $content.Replace('once_cell::sync::Lazy::new', 'std::sync::LazyLock::new')

# 2. JourneyLeg import
$content = $content.Replace('use crate::routing::JourneyLeg', 'use crate::network::JourneyLeg')

# 3. Deduplicate HashMap imports
$content = $content -replace '(?s)use std::collections::HashMap;\s*\n\s*use std::collections::HashMap;', "use std::collections::HashMap;"

# 4. Remove duplicate set_city_handler (first occurrence with SetCityRequest struct)
$pattern1 = '/// POST /api/city/set — Switch city\.\s*#\[derive\(Debug, Deserialize\)\]\s*pub struct SetCityRequest \{\s*pub city_id: String,\s*\}\s*pub\(in crate\) async fn set_city_handler\([^}]+\}\s*'
$content = $content -replace $pattern1, ''

# 5. Remove duplicate get_citizen_reports_handler (first occurrence with Query)
$pattern2 = '/// GET /api/reports — Citizen reports\.\s*pub\(in crate\) async fn get_citizen_reports_handler\(\s*Query\(query\): Query<HashMap<String, String>>,\s*\)\s*-> Json<ApiResponse<Vec<::crate::citizen_reports::CitizenReport>>> \{\s*let include_resolved = query\.get\("include_resolved"\)\.map\(|v| v == "true"\)\.unwrap_or\(false\);\s*Json\(ApiResponse::success\(::crate::citizen_reports::get_reports\(include_resolved\)\)\)\s*\}\s*'
$content = $content -replace $pattern2, ''

# 6. Remove duplicate submit_citizen_report_handler (first occurrence with Json)
$pattern3 = '/// POST /api/reports — Submit a citizen report\.\s*pub\(in crate\) async fn submit_citizen_report_handler\(\s*Json\(report\): Json<::crate::citizen_reports::CitizenReport>,\s*\)\s*-> Json<ApiResponse<String>> \{\s*let id = ::crate::citizen_reports::submit_report\(\s*&report\.report_type,\s*report\.station_id\.as_deref\(\),\s*report\.station_name\.as_deref\(\),\s*report\.lat,\s*report\.lon,\s*&report\.description,\s*\);\s*Json\(ApiResponse::success\(id\)\)\s*\}\s*'
$content = $content -replace $pattern3, ''

# 7. Remove duplicate get_alerts_handler (first occurrence)
$pattern4 = '/// GET /api/alerts — Unread smart alerts\.\s*pub\(in crate\) async fn get_alerts_handler\(\)\s*-> Json<ApiResponse<Vec<::crate::smart_alerts::AlertEvent>>> \{\s*Json\(ApiResponse::success\(::crate::smart_alerts::get_unread_alerts\(\)\)\)\s*\}\s*'
$content = $content -replace $pattern4, ''

# 8. Fix the broken if-else chain in crowd_density
$content = $content -replace 'else \{\s*"overcrowded"\.into\(\)\s*\};\s*else if new_density < 0\.85 \{\s*"very_busy"\.into\(\)\s*\}', 'else if new_density < 0.85 { "very_busy".into() } else { "overcrowded".into() };'

# 9. Add dioxus prelude to leaflet module
$content = $content -replace 'mod leaflet \{\s*use super::styles::\*;\s*use crate::logger::\*;\s*use crate::routing::roundel_svg_for_line;', "mod leaflet {\n        use super::styles::*;\n        use crate::logger::*;\n        use crate::routing::roundel_svg_for_line;\n        use dioxus::prelude::*;"

# 10. pub(crate) -> pub(in crate) but NOT on struct fields
# First, handle struct fields: pub(crate) field -> pub field (inside struct definitions)
$content = $content -replace '(?m)^(\s+)pub\(crate\)(\s+\w+\s*:)', '$1pub$2'

# 11. Then remaining pub(crate) -> pub(in crate)
$content = $content -replace 'pub\(crate\)', 'pub(in crate)'

# 12. .to_string() -> .to_owned() on string literals and &str
$content = $content -replace '\.to_string\(\)', '.to_owned()'

# 13. Add :: to crate paths in expressions (not in use statements)
$content = $content -replace '(?<![:\w])crate::', '::crate::'
$content = $content -replace 'use ::crate::', 'use crate::'

# 14. Fix std::mem, std::ptr, etc to core::
$coreMods = @('mem','ptr','fmt','cmp','convert','ops','marker','hash','iter','slice','str','ascii','ffi','hint','any','panic','num','char','f32','f64','time')
foreach ($m in $coreMods) {
    $content = $content -replace "(?<![:\w])std::$m::", "core::$m::"
}

# 15. Fix collapsible_else_if: } else { if -> } else if (single line only)
$content = $content -replace '\}\s*else\s*\{\s*if\b', '} else if'

# 16. Add :: to paths in expressions (not in use/struct/enum/trait/impl/fn/const/static/type/mod)
# Already done in #13

[System.IO.File]::WriteAllText('src/main.rs', $content, [System.Text.Encoding]::UTF8)
Write-Output "All fixes applied"