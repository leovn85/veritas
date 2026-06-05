use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

const BASE_RESOURCE_URL: &str = "https://gitlab.com/Dimbreath/turnbasedgamedata/-/raw/main";

// CÁC BẢNG BÁCH KHOA TOÀN THƯ TA CẦN ĐỂ DỊCH RELIC & LIGHT CONES
const TARGET_CONFIG_FILES: &[&str] = &[
    "EquipmentConfig.json",           // Tên Nón ánh sáng
    "RelicSetConfig.json",            // Tên bộ di vật (VD: Lượng tử, Lôi...)
    "AvatarPropertyConfig.json",      // Tên chỉ số (CRIT Rate, ATK, HP, SPD...)
    "RelicBaseType.json",             // Tên vị trí (Head, Hands, Body, Feet...)
    "AvatarConfig.json",              // Thêm tên Nhân vật (Vì Fribbels yêu cầu cả tên nhân vật)
];

fn main() {
	
	let ver = env!("CARGO_PKG_VERSION").split(".").map(|x| x.parse::<u64>().unwrap()).collect::<Vec<u64>>();
    let sem_ver = ver[0] << 48 | ver[1] << 32 | ver[1] << 16;

    winres::WindowsResource::new()
        .set_version_info(winres::VersionInfo::PRODUCTVERSION, sem_ver)
        .compile()
        .unwrap();
	
    // Đảm bảo bạn đã để file TextMapEN.json ngang hàng với Cargo.toml
    println!("cargo:rerun-if-changed=TextMapEN.json");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let mut text_hashes: HashSet<u64> = HashSet::new();

    // 1. Tải Bách khoa toàn thư về và nhặt TẤT CẢ mã Hash liên quan đến Nón, Di vật, Chỉ số
    for file_name in TARGET_CONFIG_FILES {
        let url = format!("{}/ExcelOutput/{}", BASE_RESOURCE_URL, file_name);
        // Báo cho console biết đang tải
        println!("cargo:warning=Downloading dictionary reference: {}", file_name);
        
        let json_val: serde_json::Value = ureq::get(&url)
            .call()
            .unwrap_or_else(|e| panic!("Failed to download {}: {}", url, e))
            .into_json()
            .unwrap();

        extract_hashes(&json_val, &mut text_hashes);
    }

    // 2. Đọc file TextMapEN.json CÓ SẴN TRÊN MÁY BẠN
    // =========================================================================
    // LƯU Ý: File TextMapEN.json (>50MB) không được lưu trên Git để tránh làm nặng repo.
    // Nếu build bị lỗi thiếu file, hãy tải thủ công theo đường dẫn dưới đây:
    // 🔗 Link: https://gitlab.com/Dimbreath/turnbasedgamedata/-/raw/main/TextMap/TextMapEN.json
    // Sau khi tải về, hãy đặt file này nằm ngang hàng với file Cargo.toml của dự án.
    // =========================================================================
    println!("cargo:warning=Reading local TextMapEN.json...");
    let local_textmap_path = "TextMapEN.json";
    
    let mut file = File::open(local_textmap_path)
        .expect("\n\n❌ KHÔNG TÌM THẤY FILE TextMapEN.json!\n👉 Vui lòng tải file tại: https://gitlab.com/Dimbreath/turnbasedgamedata/-/raw/main/TextMap/TextMapEN.json\n👉 Sau đó đặt file vào thư mục gốc của project (ngang hàng với Cargo.toml)\n\n");
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    
    // Parse file 50MB
    let full_text_map: std::collections::HashMap<String, String> = 
        serde_json::from_str(&contents).expect("Lỗi parse TextMapEN.json");

    // 3. Lọc: Chỉ giữ lại những dòng mà mã Hash nằm trong rổ text_hashes
    println!("cargo:warning=Filtering TextMap...");
    let mut mini_text_map: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    
    for (hash_str, text_val) in full_text_map {
        if let Ok(hash_u64) = hash_str.parse::<u64>() {
            // NẾU Hash này là tên của một cái Nón, Di vật, hoặc Chỉ số -> Giữ lại
            if text_hashes.contains(&hash_u64) {
                mini_text_map.insert(hash_u64, text_val);
            }
        }
    }

    // 4. Ghi file Mini ra thư mục OUT_DIR
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("TextMapMinimizedEN.json");
    
    let mut f = File::create(&dest_path).unwrap();
    serde_json::to_writer(&mut f, &mini_text_map).unwrap();
    
    println!("cargo:warning=Successfully created Minimized TextMap! Kept {} essential entries.", mini_text_map.len());
}

/// Hàm đệ quy: Tự động lùng sục mọi ngóc ngách của file JSON bách khoa toàn thư
/// Cứ thấy key là "Hash" và value là số, nó sẽ bỏ vào rổ.
fn extract_hashes(val: &serde_json::Value, hashes: &mut HashSet<u64>) {
    match val {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Number(num)) = map.get("Hash") {
                if let Some(hash) = num.as_u64() {
                    hashes.insert(hash);
                }
            }
            for (_, v) in map {
                extract_hashes(v, hashes);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                extract_hashes(v, hashes);
            }
        }
        _ => {}
    }
}