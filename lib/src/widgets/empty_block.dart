// 空状态：套件里的 EfEmpty 会按内容缩到文字宽，放进宽容器里像一条窄带。
// 这里统一拉满可用宽度，高度仍由内容决定。
import 'package:flutter/material.dart';

import '../../ef/widgets.dart';

class EmptyBlock extends StatelessWidget {
  const EmptyBlock({super.key, required this.text, this.action});

  final String text;
  final Widget? action;

  @override
  Widget build(BuildContext context) => SizedBox(
        width: double.infinity,
        child: EfEmpty(text: text, action: action),
      );
}
