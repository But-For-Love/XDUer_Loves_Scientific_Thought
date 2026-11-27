// 配置文件位置：显示当前用的是哪份、能不能改、怎么换。
// 路径不写死在代码里，也不靠猜，全由这里改。
import 'package:flutter/material.dart';

import '../../ef/tokens.dart';
import '../../ef/widgets.dart';
import '../state/app_state.dart';

class ConfigPathCard extends StatelessWidget {
  const ConfigPathCard({super.key, required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    final locked = state.confLocked;
    final blocked = locked || state.busy;

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.fromLTRB(16, 14, 16, 16),
      decoration: BoxDecoration(color: c.panel, border: Border.all(color: c.rule)),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const EfEyebrow('CONFIG FILE', slash: true),
              const Spacer(),
              state.conf.exists
                  ? const EfState('已存在')
                  : const EfState('还没创建', kind: EfStateKind.draft),
            ],
          ),
          const SizedBox(height: 10),
          SelectableText(
            state.confPath,
            style: EfType.ui(13, width: 100, color: c.ink, height: 1.6),
          ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 10,
            runSpacing: 10,
            children: [
              EfButton(
                '换一份…',
                small: true,
                onPressed: blocked ? null : state.pickConfFile,
              ),
              EfButton(
                '另存为…',
                small: true,
                onPressed: blocked ? null : state.saveConfAs,
              ),
              EfButton(
                '打开所在目录',
                small: true,
                onPressed: state.busy ? null : state.revealConfFile,
              ),
              EfButton(
                '恢复默认',
                small: true,
                onPressed: blocked ? null : state.useDefaultConfPath,
              ),
            ],
          ),
          const SizedBox(height: 12),
          if (locked) ...[
            const EfNotice('这次是环境变量 XDXK_CONF 指定的路径，界面上改不了。想在界面里换，先把这个环境变量去掉。'),
            const SizedBox(height: 8),
          ],
          Text(
            '默认位置：${state.defaultConfPath}',
            style: EfType.ui(12, width: 100, color: c.ink2, height: 1.6),
          ),
          const SizedBox(height: 4),
          Text(
            '换过的位置记在 ${state.appDataDir} 下的 settings.json 里',
            style: EfType.ui(12, width: 100, color: c.ink2, height: 1.6),
          ),
          const SizedBox(height: 4),
          Text(
            '界面和命令行读写的是同一份文件；改完设置点「任务」页的「保存配置」写回去。',
            style: EfType.ui(12, color: c.ink2, height: 1.6),
          ),
        ],
      ),
    );
  }
}
