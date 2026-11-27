// 深色命令栏：两套主题里都是墨色，当前分区一小块黄。
// 宽屏用它代替底部标签栏（套件里的 EfTabBar 留给窄屏）。
import 'package:flutter/material.dart';

import '../../ef/marks.dart';
import '../../ef/tokens.dart';
import '../../ef/widgets.dart';

class EfCommandBar extends StatelessWidget {
  const EfCommandBar({
    super.key,
    required this.items,
    required this.current,
    required this.onTap,
    this.trailing = const [],
    this.wordmark = '',
  });

  final List<EfTabItem> items;
  final int current;
  final ValueChanged<int> onTap;
  final List<Widget> trailing;
  final String wordmark;

  @override
  Widget build(BuildContext context) {
    return Container(
      color: Ef.commandBar,
      height: 56,
      padding: const EdgeInsets.symmetric(horizontal: Ef.pad),
      child: Row(
        children: [
          if (wordmark.isNotEmpty) ...[
            Text(
              wordmark,
              style: EfType.ui(15, weight: 800, width: 125, color: const Color(0xFFEDEDED)),
            ),
            const SizedBox(width: 18),
          ],
          Expanded(
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: [
                  for (var i = 0; i < items.length; i++) _item(context, i),
                ],
              ),
            ),
          ),
          ...trailing,
        ],
      ),
    );
  }

  Widget _item(BuildContext context, int index) {
    final active = index == current;
    const fg = Color(0xFFEDEDED);
    return Semantics(
      button: true,
      selected: active,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => onTap(index),
        child: AnimatedContainer(
          duration: Ef.d(context, Ef.wipe),
          curve: Ef.mech,
          height: 56,
          margin: const EdgeInsets.only(right: 6),
          padding: const EdgeInsets.symmetric(horizontal: 14),
          decoration: ShapeDecoration(
            color: active ? Ef.signal : Colors.transparent,
            shape: efCut(br: Ef.cutSmall),
          ),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                items[index].en.toUpperCase(),
                style: EfType.ui(9, weight: 600, width: 125, spacing: 0.14,
                    color: active ? Ef.onSignal : const Color(0xFFA3A3A3)),
              ),
              Text(
                items[index].zh,
                style: EfType.ui(14, weight: active ? 700 : 500, color: active ? Ef.onSignal : fg, height: 1.3),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 命令栏右侧的小按钮：墨底、细线框，按下变黄。
class EfBarButton extends StatelessWidget {
  const EfBarButton(this.label, {super.key, required this.onTap, this.active = false});

  final String label;
  final VoidCallback onTap;
  final bool active;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        margin: const EdgeInsets.only(left: 8),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: active ? Ef.signal : Colors.transparent,
          border: Border.all(color: active ? Ef.signal : const Color(0xFF4A4A4A)),
        ),
        child: Text(
          label,
          style: EfType.ui(12, weight: 600, width: 112, color: active ? Ef.onSignal : const Color(0xFFEDEDED)),
        ),
      ),
    );
  }
}
