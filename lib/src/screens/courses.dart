// 课程：按类别拉取课程列表，勾选后直接选课 / 退课，或把勾的课存成任务。
import 'package:flutter/material.dart';

import '../../ef/tokens.dart';
import '../../ef/widgets.dart';
import '../rust/api/dto.dart';
import '../state/app_state.dart';
import '../widgets/ef_input.dart';
import '../widgets/page.dart';
import '../widgets/empty_block.dart';

/// 学校接口里的类别代码；不认识的直接显示代码本身。
const _typeNames = {
  'BXKC': '必修课',
  'XXKC': '选修课',
  'TJKC': '通识课',
  'TYKC': '体育课',
  'XGKC': '限选课',
  'FANKC': '方案内课程',
  'XKKS': '跨专业',
};

String typeLabel(String code) => _typeNames[code] ?? code;

class CoursesScreen extends StatelessWidget {
  const CoursesScreen({super.key, required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    final visible = state.visibleCourses;

    return Column(
      children: [
        Expanded(
          child: PageBody(maxWidth: 1120, children: [
            SectionHead(
              en: 'COURSES',
              zh: '课程',
              meta: [
                EfState(
                  state.classType == null ? '未进入批次' : typeLabel(state.classType!),
                  kind: state.classType == null ? EfStateKind.draft : EfStateKind.live,
                ),
                const SizedBox(width: 8),
                Text('${visible.length} / ${state.courses.length} 行', style: EfType.field(c.ink2)),
              ],
            ),
            const SizedBox(height: 16),
            if (state.classTypes.isNotEmpty) ...[
              SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(
                  children: [
                    for (final type in state.classTypes)
                      Padding(
                        padding: const EdgeInsets.only(right: 8),
                        child: EfChip(
                          typeLabel(type),
                          active: type == state.classType,
                          onTap: state.busy ? null : () => state.selectClassType(type),
                        ),
                      ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
            ],
            _Toolbar(state: state),
            const SizedBox(height: 14),
            if (state.notice != null) ...[
              EfNotice(state.notice!),
              const SizedBox(height: 14),
            ],
            if (visible.isEmpty)
              EmptyBlock(
                text: state.classType == null
                    ? '还没有进入选课批次；先到「登录」选一个批次，这里会列出该批次的课程。'
                    : '这一类暂时没有拉到课程；点「重新拉取」再试一次。',
                action: state.classType == null
                    ? EfButton('去选批次', small: true, onPressed: () => state.setSection(1))
                    : EfButton('重新拉取', small: true, onPressed: state.busy ? null : state.reloadCourses),
              )
            else
              Container(
                decoration: BoxDecoration(border: Border(top: BorderSide(color: c.rule))),
                child: Column(
                  children: [
                    for (final row in visible)
                      _CourseLine(
                        row: row,
                        picked: state.picked.contains('${row.kch}|${row.kxh}|${row.clazzId}'),
                        onTap: () => state.togglePick(row),
                        onRaw: () => _showRaw(context, row),
                      ),
                  ],
                ),
              ),
          ]),
        ),
        _ActionBar(state: state),
      ],
    );
  }

  void _showRaw(BuildContext context, CourseRow row) {
    showEfSheet<void>(context, (_) => EfSheet(
      en: 'RAW',
      title: '${row.kch} · ${row.kcm}',
      children: [
        Container(
          margin: const EdgeInsets.symmetric(horizontal: Ef.pad),
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: EfColors.of(context).panel,
            border: Border.all(color: EfColors.of(context).rule),
          ),
          child: SelectableText(
            row.raw,
            style: EfType.ui(11, width: 100, color: EfColors.of(context).ink2, height: 1.6),
          ),
        ),
        const SizedBox(height: 12),
      ],
    ));
  }
}

class _Toolbar extends StatelessWidget {
  const _Toolbar({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 10,
      runSpacing: 10,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        SizedBox(
          width: 260,
          child: EfInput(
            label: '筛选（课程号 / 课名 / 课序号）',
            controller: state.courseQueryInput,
            hint: '24ZH2011',
            onChanged: state.setCourseQuery,
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(top: 22),
          child: Wrap(
            spacing: 10,
            runSpacing: 10,
            children: [
              EfButton('重新拉取', small: true, onPressed: state.busy ? null : state.reloadCourses),
              EfButton('全选可见', small: true, onPressed: state.pickAllVisible),
              EfButton('取消勾选', small: true, onPressed: state.clearPicked),
            ],
          ),
        ),
      ],
    );
  }
}

class _CourseLine extends StatefulWidget {
  const _CourseLine({
    required this.row,
    required this.picked,
    required this.onTap,
    required this.onRaw,
  });

  final CourseRow row;
  final bool picked;
  final VoidCallback onTap;
  final VoidCallback onRaw;

  @override
  State<_CourseLine> createState() => _CourseLineState();
}

class _CourseLineState extends State<_CourseLine> {
  bool _down = false;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    final row = widget.row;
    final on = _down || widget.picked;
    final duration = Ef.d(context, Ef.row);
    final fields = <String>[
      row.kch,
      'KXH ${row.kxh}',
      if (row.teacher.isNotEmpty) row.teacher,
      if (row.time.isNotEmpty) row.time,
      if (row.place.isNotEmpty) row.place,
      if (row.credit.isNotEmpty) '${row.credit} 学分',
    ];

    return Semantics(
      button: true,
      selected: widget.picked,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: (_) => setState(() => _down = true),
        onTapCancel: () => setState(() => _down = false),
        onTapUp: (_) {
          setState(() => _down = false);
          widget.onTap();
        },
        onLongPress: widget.onRaw,
        child: Container(
          decoration: BoxDecoration(border: Border(bottom: BorderSide(color: c.rule))),
          child: Stack(children: [
            AnimatedPositioned(
              duration: duration,
              curve: Ef.snap,
              left: 0,
              top: 0,
              bottom: on ? 0 : null,
              height: on ? null : 0,
              width: Ef.traceBar,
              child: ColoredBox(color: c.trace),
            ),
            AnimatedPadding(
              duration: duration,
              curve: Ef.snap,
              padding: EdgeInsets.fromLTRB(on ? 14 : 8, 12, 4, 12),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.center,
                children: [
                  Container(
                    width: 20,
                    height: 20,
                    decoration: BoxDecoration(
                      color: widget.picked ? c.signal : Colors.transparent,
                      border: Border.all(color: c.ink, width: 1.5),
                    ),
                    alignment: Alignment.center,
                    child: widget.picked
                        ? Text('✓', style: EfType.ui(13, weight: 800, color: Ef.onSignal))
                        : null,
                  ),
                  const SizedBox(width: 12),
                  EfTag('No.${'${row.no}'.padLeft(3, '0')}', current: widget.picked),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(row.kcm, style: EfType.text(16, weight: 500, color: c.ink, height: 1.4)),
                        const SizedBox(height: 4),
                        EfFacts(fields),
                      ],
                    ),
                  ),
                  const SizedBox(width: 12),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.end,
                    children: [
                      Text(
                        asInt(row.capacity) > 0
                            ? '${asInt(row.selected)} / ${asInt(row.capacity)}'
                            : '容量未知',
                        style: EfType.field(c.ink),
                      ),
                      const SizedBox(height: 4),
                      row.available
                          ? const EfState('有位', kind: EfStateKind.alert)
                          : const EfState('满', kind: EfStateKind.draft),
                    ],
                  ),
                ],
              ),
            ),
          ]),
        ),
      ),
    );
  }
}

class _ActionBar extends StatelessWidget {
  const _ActionBar({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      decoration: BoxDecoration(
        color: c.paper,
        border: Border(top: BorderSide(color: c.rule)),
      ),
      child: Row(
        children: [
          Text('已勾 ${state.picked.length} 门', style: EfType.ui(14, weight: 600, color: c.ink)),
          const SizedBox(width: 10),
          Text('失败自动重试：${state.autoRetry ? '开' : '关'}', style: EfType.field(c.ink2)),
          const Spacer(),
          EfButton(
            '加入任务',
            small: true,
            onPressed: state.picked.isEmpty ? null : state.addPickedAsTasks,
          ),
          const SizedBox(width: 10),
          EfButton(
            '退课',
            small: true,
            kind: EfButtonKind.danger,
            onPressed: state.busy || state.picked.isEmpty || !state.readyToPick
                ? null
                : () => state.submitPicked('del'),
          ),
          const SizedBox(width: 10),
          EfButton(
            '选课',
            kind: EfButtonKind.go,
            onPressed: state.busy || state.picked.isEmpty || !state.readyToPick
                ? null
                : () => state.submitPicked('add'),
          ),
        ],
      ),
    );
  }
}
