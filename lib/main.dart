import 'package:flutter/material.dart';

import 'src/app.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // 把 Rust 侧加载起来：之后所有 session / logs 调用都走它。
  await RustLib.init();
  runApp(const XdxkApp());
}
