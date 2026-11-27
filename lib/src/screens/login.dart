// 登录：学号密码 + 验证码（自动识别或手动输入）+ 选择批次。
import 'package:flutter/material.dart';

import '../../ef/marks.dart';
import '../../ef/tokens.dart';
import '../../ef/widgets.dart';
import '../state/app_state.dart';
import '../widgets/ef_input.dart';
import '../widgets/page.dart';
import '../widgets/empty_block.dart';

class LoginScreen extends StatelessWidget {
  const LoginScreen({super.key, required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    return PageBody(maxWidth: 760, children: [
      SectionHead(
        en: 'SIGN IN',
        zh: '登录',
        meta: [
          EfState(
            state.loggedIn ? '已登录' : '未登录',
            kind: state.loggedIn ? EfStateKind.live : EfStateKind.draft,
          ),
        ],
      ),
      const SizedBox(height: 20),
      EfPanel(
        padding: const EdgeInsets.fromLTRB(20, 20, 20, 20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            EfInput(
              label: '学号',
              controller: state.loginnameInput,
              hint: '例如 23040100000',
              monospaceDigits: true,
              enabled: !state.busy,
            ),
            const SizedBox(height: 16),
            EfInput(
              label: '密码',
              controller: state.passwordInput,
              obscure: true,
              hint: '教务系统密码',
              enabled: !state.busy,
            ),
            const SizedBox(height: 18),
            Row(
              children: [
                EfSwitch(value: state.autoOcr, onChanged: state.setAutoOcr),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('自动识别验证码', style: EfType.ui(15, weight: 600, color: c.ink)),
                      Text(
                        state.autoOcr
                            ? '取码、识别、登录自动完成，最多试三次。'
                            : '自己看图片输入四个字符。',
                        style: EfType.ui(12, color: c.ink2),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            if (!state.autoOcr) ...[
              const SizedBox(height: 18),
              _CaptchaBlock(state: state),
            ],
            const SizedBox(height: 20),
            EfButton(
              state.busy ? '正在登录…' : '登录',
              kind: EfButtonKind.go,
              expand: true,
              onPressed: state.busy ? null : state.signIn,
            ),
            if (state.loginMessage.isNotEmpty) ...[
              const SizedBox(height: 14),
              EfFacts(['账号 ${state.loginname}', state.loginMessage]),
            ],
            if (state.notice != null) ...[
              const SizedBox(height: 14),
              EfNotice(state.notice!),
            ],
          ],
        ),
      ),
      const SizedBox(height: 28),
      Row(
        children: [
          EfPair('BATCHES', '选课批次', size: 20),
          const Spacer(),
          Text('${state.batches.length} 个', style: EfType.field(c.ink2)),
        ],
      ),
      const SizedBox(height: 8),
      if (state.batches.isEmpty)
        EmptyBlock(text: '登录成功后，这里会列出这一轮开放的选课批次。')
      else
        for (var i = 0; i < state.batches.length; i++)
          EfRecord(
            no: 'No.${'${i + 1}'.padLeft(3, '0')}',
            date: state.batches[i].code,
            name: state.batches[i].name,
            meta: state.batches[i].open ? '开放中' : '未开放',
            current: i == state.batchIndex,
            onTap: state.busy ? null : () => state.enterBatch(i),
          ),
    ]);
  }
}

class _CaptchaBlock extends StatelessWidget {
  const _CaptchaBlock({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    final captcha = state.captcha;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('验证码', style: EfType.ui(13, weight: 600, color: c.ink2)),
        const SizedBox(height: 6),
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              width: 148,
              height: 52,
              decoration: BoxDecoration(color: c.panel, border: Border.all(color: c.rule)),
              child: captcha == null
                  ? EfHatch(child: Center(child: Text('还没取码', style: EfType.ui(12, color: c.ink2))))
                  : Padding(
                      padding: const EdgeInsets.all(6),
                      child: Image.memory(
                        captcha.png,
                        fit: BoxFit.contain,
                        filterQuality: FilterQuality.none,
                      ),
                    ),
            ),
            const SizedBox(width: 10),
            Column(
              children: [
                EfButton('换一张', small: true, onPressed: state.busy ? null : state.refreshCaptcha),
                const SizedBox(height: 8),
                EfButton('识别填入', small: true, onPressed: state.busy ? null : state.fillCaptchaByOcr),
              ],
            ),
            const SizedBox(width: 12),
            Expanded(
              child: EfInput(
                label: '四个字符',
                controller: state.captchaInput,
                hint: 'ABCD',
                maxLength: 4,
                monospaceDigits: true,
                onSubmitted: (_) => state.signIn(),
              ),
            ),
          ],
        ),
        const SizedBox(height: 6),
        Text(
          '看不清就点「换一张」；识别结果不对可以直接改。',
          style: EfType.ui(12, color: c.ink2),
        ),
      ],
    );
  }
}
