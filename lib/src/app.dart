// 应用外壳：顶部三色条、宽屏深色命令栏 / 窄屏底部标签栏、页面切换。
import 'dart:async';

import 'package:flutter/material.dart';

import '../ef/marks.dart';
import '../ef/tokens.dart';
import '../ef/widgets.dart';
import 'screens/courses.dart';
import 'screens/login.dart';
import 'screens/logs.dart';
import 'screens/overview.dart';
import 'screens/tasks.dart';
import 'state/app_state.dart';
import 'widgets/command_bar.dart';

const _sections = <EfTabItem>[
  EfTabItem('Overview', '概览'),
  EfTabItem('Sign in', '登录'),
  EfTabItem('Courses', '课程'),
  EfTabItem('Tasks', '任务'),
  EfTabItem('Logs', '日志'),
];

class XdxkApp extends StatefulWidget {
  const XdxkApp({super.key});

  @override
  State<XdxkApp> createState() => _XdxkAppState();
}

class _XdxkAppState extends State<XdxkApp> {
  late final AppState _state = AppState();
  final ValueNotifier<ThemeMode> _themeMode = ValueNotifier(ThemeMode.system);

  @override
  void initState() {
    super.initState();
    _state.bootstrap();
  }

  @override
  void dispose() {
    _state.dispose();
    _themeMode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<ThemeMode>(
      valueListenable: _themeMode,
      builder: (context, mode, _) => MaterialApp(
        title: '西电选课',
        debugShowCheckedModeBanner: false,
        theme: efTheme(Brightness.light),
        darkTheme: efTheme(Brightness.dark),
        themeMode: mode,
        home: AnimatedBuilder(
          animation: _state,
          builder: (context, _) => Shell(state: _state, themeMode: _themeMode),
        ),
      ),
    );
  }
}

class Shell extends StatelessWidget {
  const Shell({super.key, required this.state, required this.themeMode});

  final AppState state;
  final ValueNotifier<ThemeMode> themeMode;

  @override
  Widget build(BuildContext context) {
    final wide = MediaQuery.sizeOf(context).width >= 900;
    final pages = <Widget>[
      OverviewScreen(state: state),
      LoginScreen(state: state),
      CoursesScreen(state: state),
      TasksScreen(state: state),
      LogsScreen(state: state),
    ];

    return Scaffold(
      body: SafeArea(
        bottom: !wide,
        child: Column(
          children: [
            const EfTriStrip(),
            if (wide) _commandBar(context) else const SizedBox(height: 0),
            if (state.busy) SizedBox(height: 3, child: EfHatch(child: const SizedBox.expand())),
            Expanded(
              child: Stack(
                children: [
                  Positioned.fill(child: EfContours(child: const SizedBox.expand())),
                  Positioned.fill(child: pages[state.section.clamp(0, pages.length - 1)]),
                  Positioned(
                    left: 0,
                    right: 0,
                    bottom: wide ? 12 : 0,
                    child: _ToastHost(text: state.toast, onDone: state.clearToast),
                  ),
                ],
              ),
            ),
            if (!wide) EfTabBar(items: _sections, current: state.section, onTap: state.setSection),
          ],
        ),
      ),
    );
  }

  Widget _commandBar(BuildContext context) {
    return Column(
      children: [
        EfCommandBar(
          wordmark: 'XDXK',
          items: _sections,
          current: state.section,
          onTap: state.setSection,
          trailing: [
            ValueListenableBuilder<ThemeMode>(
              valueListenable: themeMode,
              builder: (context, mode, _) {
                final dark = mode == ThemeMode.dark;
                return EfBarButton(
                  dark ? '浅色' : '深色',
                  active: dark,
                  onTap: () => themeMode.value = dark ? ThemeMode.light : ThemeMode.dark,
                );
              },
            ),
            Padding(
              padding: const EdgeInsets.only(left: 12),
              child: Text(
                state.busy ? '处理中' : (state.loggedIn ? '已登录' : '未登录'),
                style: EfType.ui(12, weight: 600, width: 112,
                    color: state.busy ? Ef.signal : const Color(0xFFA3A3A3)),
              ),
            ),
          ],
        ),
      ],
    );
  }
}

/// 提示条：出现 2 秒后自己擦掉。
class _ToastHost extends StatefulWidget {
  const _ToastHost({required this.text, required this.onDone});

  final String text;
  final VoidCallback onDone;

  @override
  State<_ToastHost> createState() => _ToastHostState();
}

class _ToastHostState extends State<_ToastHost> {
  Timer? _timer;

  @override
  void didUpdateWidget(covariant _ToastHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.text.isNotEmpty && widget.text != oldWidget.text) {
      _timer?.cancel();
      _timer = Timer(const Duration(milliseconds: 2400), widget.onDone);
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (widget.text.isEmpty) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: EfToast(widget.text),
        ),
      ),
    );
  }
}
