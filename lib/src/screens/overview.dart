// 概览：一眼看到「登没登、有没有批次、配了几条任务、日志有没有异常」。
import 'package:flutter/material.dart';

import '../../ef/tokens.dart';
import '../../ef/widgets.dart';
import '../rust/api/dto.dart';
import '../state/app_state.dart';
import '../widgets/config_card.dart';
import '../widgets/page.dart';
import '../widgets/empty_block.dart';

class OverviewScreen extends StatelessWidget {
  const OverviewScreen({super.key, required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    final batch = state.batchIndex >= 0 && state.batchIndex < state.batches.length
        ? state.batches[state.batchIndex]
        : null;
    final wide = MediaQuery.sizeOf(context).width >= 820;

    return PageBody(children: [
      SectionHead(
        en: 'OVERVIEW',
        zh: '概览',
        meta: [
          EfState(
            state.loggedIn ? '已登录' : '未登录',
            kind: state.loggedIn ? EfStateKind.live : EfStateKind.draft,
          ),
          const SizedBox(width: 8),
          EfState(
            state.running ? '任务进行中' : '待机',
            kind: state.running ? EfStateKind.alert : EfStateKind.live,
          ),
        ],
      ),
      const SizedBox(height: 20),
      _Figures(state: state),
      const SizedBox(height: 24),
      if (state.notice != null) ...[
        EfNotice(state.notice!),
        const SizedBox(height: 20),
      ],
      if (wide)
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(flex: 3, child: _BatchPanel(state: state, batch: batch)),
            const SizedBox(width: 20),
            Expanded(flex: 2, child: _NextPanel(state: state)),
          ],
        )
      else ...[
        _BatchPanel(state: state, batch: batch),
        const SizedBox(height: 20),
        _NextPanel(state: state),
      ],
      const SizedBox(height: 24),
      EfBarTitle('配置文件'),
      const SizedBox(height: 10),
      ConfigPathCard(state: state),
      const SizedBox(height: 24),
      EfBarTitle('环境'),
      const SizedBox(height: 10),
      Text(state.modelStatus, style: EfType.ui(13, color: c.ink2, height: 1.6)),
      const SizedBox(height: 4),
      Text(
        '账号密码只用于登录教务系统，不会上传到别处；保存配置时写进上面这份文件。',
        style: EfType.text(14, color: c.ink2),
      ),
    ]);
  }
}

class _Figures extends StatelessWidget {
  const _Figures({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    final items = <(String, String, bool)>[
      ('${state.batches.length}', '可选批次', false),
      ('${state.courses.length}', '课程行', false),
      ('${state.tasks.length}', '任务', state.tasks.isNotEmpty),
      ('${state.logs.length}', '日志行', false),
    ];
    return Container(
      decoration: BoxDecoration(color: c.panel, border: Border.all(color: c.rule)),
      padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 18),
      child: Row(
        children: [
          for (var i = 0; i < items.length; i++)
            Expanded(
              child: Container(
                padding: EdgeInsets.only(left: i == 0 ? 0 : 16),
                decoration: BoxDecoration(
                  border: i == 0 ? null : Border(left: BorderSide(color: c.rule)),
                ),
                child: EfFigure(items[i].$1, items[i].$2, alert: items[i].$3),
              ),
            ),
        ],
      ),
    );
  }
}

class _BatchPanel extends StatelessWidget {
  const _BatchPanel({required this.state, required this.batch});

  final AppState state;
  final BatchView? batch;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    return EfPanel(
      padding: const EdgeInsets.fromLTRB(18, 18, 18, 18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const EfEyebrow('BATCH', slash: true),
          const SizedBox(height: 8),
          if (batch != null) ...[
            Text(batch!.name, style: EfType.ui(20, weight: 700, color: c.ink, height: 1.4)),
            const SizedBox(height: 8),
            EfFacts([
              batch!.code,
              batch!.open ? '开放中' : '未开放',
              '${state.classTypes.length} 个课程类别',
            ]),
            const SizedBox(height: 16),
            EfButton(
              state.classTypes.isEmpty ? '进入这个批次' : '重新进入批次',
              kind: EfButtonKind.go,
              onPressed: state.busy ? null : () => state.enterBatch(state.batchIndex),
            ),
          ] else if (state.loggedIn)
            EmptyBlock(
              text: '登录成功了，但这个账号没有可选批次。',
              action: EfButton('重新登录', small: true, onPressed: () => state.setSection(1)),
            )
          else
            EmptyBlock(
              text: '还没有登录；登录后这里会显示这一轮能选的批次。',
              action: EfButton('去登录', small: true, onPressed: () => state.setSection(1)),
            ),
        ],
      ),
    );
  }
}

class _NextPanel extends StatelessWidget {
  const _NextPanel({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    return EfPanel(
      cutBr: Ef.cutPanel,
      padding: const EdgeInsets.fromLTRB(0, 16, 0, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 18),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const EfEyebrow('NEXT', slash: true),
                const SizedBox(height: 8),
                Text('按顺序做三件事', style: EfType.ui(18, weight: 700, color: c.ink)),
              ],
            ),
          ),
          const SizedBox(height: 12),
          EfActionRow('1. 登录教务系统', meta: state.loggedIn ? '已完成' : '待做', onTap: () => state.setSection(1)),
          EfActionRow(
            '2. 进入批次并拉课程',
            meta: state.courses.isEmpty ? '待做' : '${state.courses.length} 行',
            onTap: () => state.setSection(state.loggedIn ? 2 : 1),
          ),
          EfActionRow(
            '3. 配任务并开抢',
            meta: state.tasks.isEmpty ? '待做' : '${state.tasks.length} 条',
            onTap: () => state.setSection(3),
          ),
        ],
      ),
    );
  }
}
