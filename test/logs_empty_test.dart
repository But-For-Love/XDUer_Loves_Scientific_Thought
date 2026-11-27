// 回归测试：日志页没有日志时，空状态是一条满宽的窄带，而不是被撑成一根竖条。
// 这个测试不碰 Rust：日志为空时界面不会调用桥接层。
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:xdxk_gui/ef/tokens.dart';
import 'package:xdxk_gui/ef/widgets.dart';
import 'package:xdxk_gui/src/screens/logs.dart';
import 'package:xdxk_gui/src/state/app_state.dart';

void main() {
  testWidgets('日志空状态宽度拉满、高度只占内容', (tester) async {
    final state = AppState();
    addTearDown(state.dispose);
    await tester.binding.setSurfaceSize(const Size(1200, 800));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    await tester.pumpWidget(
      MaterialApp(
        theme: efTheme(Brightness.light),
        home: Scaffold(
          body: LogsScreen(state: state),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final empty = find.byType(EfEmpty);
    expect(empty, findsOneWidget);

    final size = tester.getSize(empty);
    // 满宽（左右各有 16 的内边距）
    expect(size.width, greaterThan(1100));
    // 高度只由文字决定，不应该撑满 620 的可用高度
    expect(size.height, lessThan(160));
  });
}
