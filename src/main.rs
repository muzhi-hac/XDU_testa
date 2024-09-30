use eframe::egui;
use serialport::{self, SerialPort};
use std::io:: Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct SerialGuiApp {
    student_id: String,
    humidity_value: Arc<Mutex<String>>,  // 修改为线程安全的共享字符串
    port_list: Vec<String>,
    selected_port: Option<String>,
    port: Option<Arc<Mutex<Box<dyn SerialPort>>>>,
    is_port_open: bool,
}

impl Default for SerialGuiApp {
    fn default() -> Self {
        Self {
            student_id: String::new(),
            humidity_value: Arc::new(Mutex::new(String::new())),  // 初始化为新的共享字符串
            port_list: serialport::available_ports().map(|ports| ports.into_iter().map(|p| p.port_name).collect()).unwrap_or_else(|_| vec![]),
            selected_port: None,
            port: None,
            is_port_open: false,
        }
    }
}


impl eframe::App for SerialGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Student ID Serial and humidity GUI");

            // 学号输入框
            ui.horizontal(|ui| {
                ui.label("Student ID:");
                ui.text_edit_singleline(&mut self.student_id);
            });

            // 串口选择
            ui.horizontal(|ui| {
                ui.label("Serial Port:");
                egui::ComboBox::from_label("")
                    .selected_text(self.selected_port.clone().unwrap_or_else(|| "Serial Port".to_string()))
                    .show_ui(ui, |ui| {
                        for port in &self.port_list {
                            ui.selectable_value(&mut self.selected_port, Some(port.clone()), port);
                        }
                    });
            });

            // 打开/关闭串口
            ui.horizontal(|ui| {
                let button_text = if self.is_port_open { "close port" } else { "open port" };
                if ui.button(button_text).clicked() {
                    if self.is_port_open {
                        self.close_port();
                    } else {
                        if let Some(port_name) = &self.selected_port {
                            self.open_port(&port_name.clone());
                        } else {
                            // 可以在这里添加错误提示，例如：没有选择串口
                            eprintln!("please select a serial port first");
                        }
                    }
                }
            });
            

            // 发送按钮
            if ui.button("send student id").clicked() {
                if self.is_port_open {
                    self.send_student_id();
                } else {
                    ui.label("please open a serial port first");
                }
            }

            // 显示发送的学号
            ui.label(format!("sended id: {}", self.student_id));

            // 显示接收到的湿度值
            let humidity = self.humidity_value.lock().unwrap().clone();
            ui.label(format!("Received humidity: {}%", humidity));
        });

        ctx.request_repaint(); // 保证 UI 刷新
    }
}

impl SerialGuiApp {
    // 打开串口
    fn open_port(&mut self, port_name: &str) {
        match serialport::new(port_name, 9600)
            .timeout(Duration::from_millis(1000))
            .open()
        {
            Ok(port) => {
                self.port = Some(Arc::new(Mutex::new(port)));
                self.is_port_open = true;
                self.start_receiving_data();
            }
            Err(e) => {
                eprintln!("fail to open port: {}", e);
            }
        }
    }

    // 关闭串口
    fn close_port(&mut self) {
        self.port = None;
        self.is_port_open = false;
    }

    // 发送学号
    fn send_student_id(&mut self) {
        if let Some(port) = &self.port {
            let mut port = port.lock().unwrap();
            let _ = port.write(self.student_id.as_bytes());
        }
    }

    fn start_receiving_data(&mut self) {
        let humidity_value = self.humidity_value.clone();
        if let Some(port) = self.port.clone() {
            thread::spawn(move || {
                let mut accumulated_data = String::new();
                let mut buf: Vec<u8> = vec![0; 1024];
                let mut timeout_count = 0;  // 超时计数器
                loop {
                    println!("reading data");
                    let mut port = port.lock().unwrap();
                    match port.read(buf.as_mut_slice()) {
                        Ok(n) if n > 0 => {
                            println!("Received data: {:?}", &buf[..n]);
                            let part = String::from_utf8_lossy(&buf[..n]);
                            accumulated_data.push_str(&part);
                            println!("Accumulated data: {}", accumulated_data);
                            while let Some(end_idx) = accumulated_data.find('%') {
                                let next_idx = end_idx + 1;
                
                                let complete_data = &accumulated_data[..next_idx];
                                println!("Complete data1: {}", complete_data);
                                let received_humidity_value = complete_data.split(':').nth(1).unwrap_or("").trim().trim_end_matches('%').trim();
                                println!("Received humidity2: {}", received_humidity_value);
                                let mut humidity = humidity_value.lock().unwrap();
                                *humidity = received_humidity_value.to_string();
                                // if complete_data.starts_with("Humidity:") {
                                //     let received_humidity_value = complete_data.split(':').nth(1).unwrap_or("").trim().trim_end_matches('%').trim();
                                //     println!("Received humidity2: {}", received_humidity_value);
                                //     let mut humidity = humidity_value.lock().unwrap();
                                //     *humidity = received_humidity_value.to_string();
                                //     println!("Received humidity3: {}", received_humidity_value);
                                // }
                                accumulated_data = accumulated_data[next_idx..].to_string();
                            }
                            timeout_count = 0;  // 重置超时计数器
                        }
                        Ok(_) => {
                            // 这里没有读取到数据
                        }
                        Err(e) => {
                            eprintln!("Read error: {}", e);
                            if e.kind() == std::io::ErrorKind::TimedOut {
                                timeout_count += 1;
                                if timeout_count > 5 {  // 例如连续五次超时则退出
                                    break;
                                }
                            } else {
                                break;  // 对于非超时错误，仍然退出
                            }
                        }
                    }
                    thread::sleep(std::time::Duration::from_secs(1));
                }
            });
        }
    }
    
    
    
}

fn main() -> Result<(), eframe::Error> {
    let humidity_value = Arc::new(Mutex::new(0.0));


    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Student ID Serial and humidity GUI",
        options,
        Box::new(|_cc| Box::new(SerialGuiApp::default())),
    )
}
