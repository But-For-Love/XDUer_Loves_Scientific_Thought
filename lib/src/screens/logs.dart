// 日志：Rust 侧的每一次请求、重试、结果都会落到这里。
import 'package:flutter/material.dart';

import '../../ef/tokens.dart';
import '../../ef/widgets.dart';
import '../rust/api/logs.dart';
import '../state/app_state.dart';
import '../widgets/page.dart';
import '../widgets/empty_block.dart';

class LogsScreen extends StatefulWidget {
  const LogsScreen({super.key, required this.state});

  final AppState state;

  @override
  State<LogsScreen> createState() => _LogsScreenState();
}

class _LogsScreenState extends State<LogsScreen> {
  final ScrollController _scroll = ScrollController();
  bool _stickToBottom = true;
  int _lastCount = 0;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(() {
      if (!_scroll.hasClients) return;
      _stickToBottom = _scroll.position.pixels >= _scroll.position.maxScrollExtent - 48;
    });
  }

  @override
  void dispose() {
    _scroll.dispose();
    super.dispose();
  }

  void _keepAtBottom() {
    final lines = widget.state.visibleLogs.length;
    if (lines == _lastCount) return;
    _lastCount = lines;
    if (!_stickToBottom) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_scroll.hasClients) return;
      _scroll.jumpTo(_scroll.position.maxScrollExtent);
    });
  }

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    final state = widget.state;
    final lines = state.visibleLogs;
    _keepAtBottom();

    return Column(
      children: [
        Padding(
          padding: EdgeInsets.fromLTRB(
            MediaQuery.sizeOf(context).width >= 900 ? 32 : 16,
            20,
            MediaQuery.sizeOf(context).width >= 900 ? 32 : 16,
            12,
          ),
          child: Column(
            children: [
              SectionHead(
                en: 'LOGS',
                zh: '日志',
                meta: [
                  if (state.running) ...[
                    const EfState('执行中', kind: EfStateKind.alert),
                    const SizedBox(width: 8),
                  ],
                  Text('${lines.length} 行 │ 异常 ${state.problemLogs}',
                      style: EfType.field(c.ink2)),
                ],
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  EfSwitch(value: state.onlyProblems, onChanged: (_) => state.toggleOnlyProblems()),
                  const SizedBox(width: 10),
                  Text('只看异常', style: EfType.ui(14, weight: 600, color: c.ink)),
                  const Spacer(),
                  EfButton('清空', small: true, onPressed: state.clearLogs),
                ],
              ),
            ],
          ),
        ),
        Expanded(
          child: lines.isEmpty
              ? SingleChildScrollView(
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  child: EmptyBlock(
                    text: state.onlyProblems
                        ? '暂时没有异常：登录、请求、重试都正常。'
                        : '登录、请求、重试和结果都会记在这里；现在还没有记录。',
                  ),
                )
              : SelectionArea(
                  child: ListView.builder(
                    controller: _scroll,
                    padding: const EdgeInsets.only(bottom: 24),
                    itemCount: lines.length,
                    itemBuilder: (context, index) => _LogRow(line: lines[index], index: index),
                  ),
                ),
        ),
      ],
    );
  }
}

class _LogRow extends StatelessWidget {
  const _LogRow({required this.line, required this.index});

  final LogLine line;
  final int index;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    final problem = line.level == 'ERROR' || line.level == 'WARN';
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 7),
      decoration: BoxDecoration(border: Border(bottom: BorderSide(color: c.rule))),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 84,
            child: Text(line.time, style: EfType.ui(11, width: 112, color: c.ink2)),
          ),
          Container(
            width: 42,
            alignment: Alignment.center,
            color: problem ? c.signal : null,
            child: Text(
              line.level == 'ERROR' ? 'ERR' : line.level == 'WARN' ? 'WRN' : 'INF',
              style: EfType.ui(10, weight: 700, width: 112,
                  color: problem ? Ef.onSignal : c.ink2),
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              line.message,
              style: EfType.ui(13, color: c.ink, height: 1.5),
            ),
          ),
          const SizedBox(width: 10),
          if (MediaQuery.sizeOf(context).width >= 760)
            Text(line.target, style: EfType.ui(10, width: 112, color: c.ink2)),
        ],
      ),
    );
  }
}
