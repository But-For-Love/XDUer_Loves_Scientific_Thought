// 任务：把「要抢哪门课」写成列表，存进 conf.json，然后一键开抢。
import 'package:flutter/material.dart';

import '../../ef/tokens.dart';
import '../../ef/widgets.dart';
import '../rust/api/dto.dart';
import '../state/app_state.dart';
import '../widgets/ef_input.dart';
import '../widgets/page.dart';
import '../widgets/empty_block.dart';

class TasksScreen extends StatelessWidget {
  const TasksScreen({super.key, required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    return PageBody(maxWidth: 1000, children: [
      SectionHead(
        en: 'TASKS',
        zh: '任务',
        meta: [
          if (state.running) ...[
            const EfState('执行中', kind: EfStateKind.alert),
            const SizedBox(width: 8),
          ],
          Text('${state.tasks.length} 条', style: EfType.field(c.ink2)),
        ],
      ),
      const SizedBox(height: 16),
      if (state.notice != null) ...[
        EfNotice(state.notice!),
        const SizedBox(height: 16),
      ],
      _AddForm(state: state),
      const SizedBox(height: 22),
      Row(
        children: [
          EfPair('QUEUE', '任务列表', size: 20),
          const Spacer(),
          Text('必修 ${state.tasks.where((task) => task.category == 0).length} 条 │ '
              '选修 ${state.tasks.where((task) => task.category == 1).length} 条',
              style: EfType.field(c.ink2)),
        ],
      ),
      const SizedBox(height: 8),
      if (state.tasks.isEmpty)
        EmptyBlock(
          text: '还没有任务。在上面的表单里填课程号和课序号加一条，或者到「课程」里勾几门课加入任务。',
          action: EfButton('去课程列表', small: true, onPressed: () => state.setSection(2)),
        )
      else
        Container(
          decoration: BoxDecoration(border: Border(top: BorderSide(color: c.rule))),
          child: Column(
            children: [
              for (var i = 0; i < state.tasks.length; i++) _TaskLine(state: state, index: i),
            ],
          ),
        ),
      const SizedBox(height: 22),
      _Controls(state: state),
      if (state.inspectLines.isNotEmpty) ...[
        const SizedBox(height: 22),
        const EfEyebrow('DIAGNOSIS', slash: true),
        const SizedBox(height: 8),
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(color: c.panel, border: Border.all(color: c.rule)),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              for (final line in state.inspectLines)
                Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Text(line, style: EfType.ui(13, width: 100, color: c.ink, height: 1.6)),
                ),
            ],
          ),
        ),
      ],
    ]);
  }
}

class _AddForm extends StatelessWidget {
  const _AddForm({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    return EfPanel(
      padding: const EdgeInsets.fromLTRB(18, 16, 18, 18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const EfEyebrow('NEW TASK', slash: true),
          const SizedBox(height: 10),
          Wrap(
            spacing: 14,
            runSpacing: 14,
            children: [
              SizedBox(
                width: 200,
                child: EfInput(
                  label: '课程号 KCH',
                  controller: state.taskKch,
                  hint: '24ZH2011',
                  monospaceDigits: true,
                ),
              ),
              SizedBox(
                width: 160,
                child: EfInput(
                  label: '课序号 KXH',
                  controller: state.taskKxh,
                  hint: '01',
                  monospaceDigits: true,
                ),
              ),
              SizedBox(
                width: 220,
                child: EfInput(
                  label: '课名（只用于日志）',
                  controller: state.taskKcm,
                  hint: '可以不填',
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          Wrap(
            spacing: 16,
            runSpacing: 12,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              SizedBox(
                width: 220,
                child: EfSegmented(
                  options: const ['必修', '选修'],
                  current: state.taskCategory,
                  onChanged: state.setTaskCategory,
                ),
              ),
              SizedBox(
                width: 220,
                child: EfSegmented(
                  options: const ['选课', '退课'],
                  current: state.taskAction == 'del' ? 1 : 0,
                  onChanged: (index) => state.taskAction = index == 1 ? 'del' : 'add',
                ),
              ),
              EfButton('加入列表', small: true, onPressed: state.addTask),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            '课序号要跟教务系统里完全一致：区分大小写，前导零不能省（02 不等于 2）。',
            style: EfType.ui(12, color: c.ink2),
          ),
        ],
      ),
    );
  }
}

class _TaskLine extends StatelessWidget {
  const _TaskLine({required this.state, required this.index});

  final AppState state;
  final int index;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    final TaskView task = state.tasks[index];
    return Container(
      padding: const EdgeInsets.symmetric(vertical: 10, horizontal: 8),
      decoration: BoxDecoration(border: Border(bottom: BorderSide(color: c.rule))),
      child: Row(
        children: [
          EfTag('No.${'${index + 1}'.padLeft(3, '0')}'),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  task.kcm.isEmpty ? '（没有填课名）' : task.kcm,
                  style: EfType.text(16, weight: 500, color: c.ink, height: 1.4),
                ),
                const SizedBox(height: 4),
                EfFacts([
                  task.kch,
                  'KXH ${task.kxh}',
                  task.category == 0 ? '必修' : '选修',
                  switch (task.action) {
                    'del' => '退课',
                    'query' => '只查询',
                    _ => '选课',
                  },
                ]),
              ],
            ),
          ),
          const SizedBox(width: 8),
          EfChip(
            '选课',
            active: task.action != 'del',
            onTap: () => state.setTaskAction(index, 'add'),
          ),
          const SizedBox(width: 6),
          EfChip(
            '退课',
            active: task.action == 'del',
            onTap: () => state.setTaskAction(index, 'del'),
          ),
          const SizedBox(width: 10),
          EfButton('删除', small: true, kind: EfButtonKind.danger, onPressed: () => state.removeTask(index)),
        ],
      ),
    );
  }
}

class _Controls extends StatelessWidget {
  const _Controls({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(color: c.panel, border: Border.all(color: c.rule)),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              EfSwitch(value: state.autoRetry, onChanged: state.setAutoRetry),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('失败自动重试', style: EfType.ui(15, weight: 600, color: c.ink)),
                    Text(
                      state.autoRetry
                          ? '没抢到会一直重试，直到成功或你点停止。'
                          : '每个任务只提交一次。',
                      style: EfType.ui(12, color: c.ink2),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          Wrap(
            spacing: 10,
            runSpacing: 10,
            children: [
              EfButton(
                state.running ? '执行中…' : '开始执行',
                kind: EfButtonKind.go,
                onPressed: state.busy || state.running ? null : state.runAllTasks,
              ),
              EfButton('停止', onPressed: state.running ? state.stopRun : null),
              EfButton('保存配置', onPressed: state.busy ? null : state.saveConfig),
              EfButton('只读诊断', small: true, onPressed: state.busy ? null : state.runInspect),
            ],
          ),
          const SizedBox(height: 10),
          Text(
            '保存配置 → ${state.confPath}',
            style: EfType.ui(12, width: 100, color: c.ink2, height: 1.6),
          ),
          const SizedBox(height: 4),
          Text(
            '执行时会先拉一遍当前批次的课程列表，找到每一门课的课序号再提交；找不到的会在日志里写明。',
            style: EfType.ui(12, color: c.ink2, height: 1.6),
          ),
        ],
      ),
    );
  }
}
