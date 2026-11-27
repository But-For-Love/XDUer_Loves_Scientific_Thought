// 页面骨架：统一的边距、最大宽度、分区标题。
import 'package:flutter/material.dart';

import '../../ef/tokens.dart';
import '../../ef/widgets.dart';

/// 内容容器：窄屏贴边，宽屏居中并限宽，避免长行。
class PageBody extends StatelessWidget {
  const PageBody({super.key, required this.children, this.maxWidth = 1040, this.padding});

  final List<Widget> children;
  final double maxWidth;
  final EdgeInsets? padding;

  @override
  Widget build(BuildContext context) {
    final pad = padding ??
        EdgeInsets.fromLTRB(
          MediaQuery.sizeOf(context).width >= 900 ? 32 : 16,
          20,
          MediaQuery.sizeOf(context).width >= 900 ? 32 : 16,
          32,
        );
    return SingleChildScrollView(
      padding: pad,
      child: Center(
        child: ConstrainedBox(
          constraints: BoxConstraints(maxWidth: maxWidth),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: children,
          ),
        ),
      ),
    );
  }
}

/// 分区标题：// ENGLISH 在上，中文粗体在下，右侧放真数据。
class SectionHead extends StatelessWidget {
  const SectionHead({super.key, required this.en, required this.zh, this.meta = const [], this.size = 24});

  final String en;
  final String zh;
  final List<Widget> meta;
  final double size;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    return Container(
      padding: const EdgeInsets.only(bottom: 12),
      decoration: BoxDecoration(border: Border(bottom: BorderSide(color: c.rule))),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          EfPair(en, zh, size: size),
          const Spacer(),
          ...meta,
        ],
      ),
    );
  }
}

/// 竖线分隔的一行小字，例如「2026.11.27 │ 3 条任务」。
class MetaLine extends StatelessWidget {
  const MetaLine(this.items, {super.key});

  final List<String> items;

  @override
  Widget build(BuildContext context) => EfFacts(items);
}
