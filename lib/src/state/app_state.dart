// 界面状态：把 Rust 侧的会话包装成一组可以直接绑到控件上的字段。
// 所有跨语言调用都在这里发生，屏幕只读字段、调方法。
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';

import '../rust/api.dart' as bridge;
import '../rust/api/dto.dart';
import '../rust/api/logs.dart';
import '../rust/api/session.dart' as rust;
import '../rust/api/settings.dart' as paths;

/// PlatformInt64 在桌面 / 移动端就是 int，在 web 上是 BigInt。
int asInt(Object? value) {
  if (value is int) return value;
  if (value == null) return 0;
  return int.tryParse(value.toString()) ?? 0;
}

String describeError(Object error) {
  if (error is AnyhowException) return error.message;
  if (error is PanicException) return error.message;
  return error.toString();
}

ConfView emptyConf() => ConfView(
      path: '',
      exists: false,
      loginname: '',
      password: '',
      batch: '',
      ocrCaptcha: true,
      always: true,
      intervalMs: BigInt.zero,
      tasks: const [],
      note: '',
    );

class AppState extends ChangeNotifier {
  AppState();

  // ── 导航与全局状态 ────────────────────────────────────────────────
  int section = 0;
  bool busy = false;
  String? notice;
  String toast = '';

  // ── 配置 ────────────────────────────────────────────────────────
  ConfView conf = emptyConf();
  String confPath = '';
  String defaultConfPath = '';
  String appDataDir = '';
  bool confLocked = false;
  String modelStatus = '';
  final TextEditingController loginnameInput = TextEditingController();
  final TextEditingController passwordInput = TextEditingController();
  bool autoOcr = true;
  bool autoRetry = true;
  List<TaskView> tasks = [];

  // ── 登录 ────────────────────────────────────────────────────────
  CaptchaView? captcha;
  final TextEditingController captchaInput = TextEditingController();
  List<BatchView> batches = [];
  int batchIndex = -1;
  bool loggedIn = false;
  String loginMessage = '';

  // ── 课程 ────────────────────────────────────────────────────────
  List<String> classTypes = [];
  String? classType;
  List<CourseRow> courses = [];
  final Set<String> picked = {};
  final TextEditingController courseQueryInput = TextEditingController();
  List<String> inspectLines = [];

  // ── 日志 ────────────────────────────────────────────────────────
  final List<LogLine> logs = [];
  bool onlyProblems = false;
  bool running = false;

  // 任务表单草稿
  final TextEditingController taskKch = TextEditingController();
  final TextEditingController taskKxh = TextEditingController();
  final TextEditingController taskKcm = TextEditingController();
  int taskCategory = 1;
  String taskAction = 'add';

  bool _disposed = false;
  StreamSubscription<LogLine>? _logSub;

  @override
  void dispose() {
    _disposed = true;
    _logSub?.cancel();
    captchaInput.dispose();
    courseQueryInput.dispose();
    loginnameInput.dispose();
    passwordInput.dispose();
    taskKch.dispose();
    taskKxh.dispose();
    taskKcm.dispose();
    super.dispose();
  }

  void _notify() {
    if (!_disposed) notifyListeners();
  }

  // ── 派生数据 ────────────────────────────────────────────────────

  String get loginname => loginnameInput.text;
  String get password => passwordInput.text;

  List<CourseRow> get visibleCourses {
    final query = courseQuery.trim().toLowerCase();
    if (query.isEmpty) return courses;
    return courses
        .where((row) =>
            row.kch.toLowerCase().contains(query) ||
            row.kcm.toLowerCase().contains(query) ||
            row.kxh.toLowerCase().contains(query))
        .toList();
  }

  String get courseQuery => courseQueryInput.text;

  List<CourseRow> get pickedRows =>
      courses.where((row) => picked.contains(_key(row))).toList();

  bool get readyToPick => loggedIn && classType != null;

  String _key(CourseRow row) => '${row.kch}|${row.kxh}|${row.clazzId}';

  List<LogLine> get visibleLogs =>
      onlyProblems ? logs.where((line) => line.level != 'INFO' && line.level != 'DEBUG').toList() : logs;

  int get problemLogs => logs.where((line) => line.level != 'INFO' && line.level != 'DEBUG').length;

  // ── 启动 ────────────────────────────────────────────────────────

  Future<void> bootstrap() async {
    confPath = paths.confPath();
    defaultConfPath = paths.defaultConfPath();
    appDataDir = paths.appDataDir();
    confLocked = paths.confLocked();
    modelStatus = bridge.modelStatus();
    conf = bridge.loadConfig();
    loginnameInput.text = conf.loginname;
    passwordInput.text = conf.password;
    autoOcr = conf.ocrCaptcha;
    autoRetry = conf.always;
    tasks = List.of(conf.tasks);
    if (conf.note.isNotEmpty) notice = conf.note;
    _logSub = logStream().listen(
      _onLog,
      onError: (Object error) {
        notice = '日志流断开：${describeError(error)}';
        _notify();
      },
    );
    _notify();
  }

  void _onLog(LogLine line) {
    logs.add(line);
    if (logs.length > 600) logs.removeRange(0, logs.length - 600);
    _notify();
  }

  Future<void> _run(Future<void> Function() job) async {
    if (busy) return;
    busy = true;
    notice = null;
    _notify();
    try {
      await job();
    } catch (error) {
      notice = describeError(error);
    } finally {
      busy = false;
      _notify();
    }
  }

  void setSection(int index) {
    section = index;
    notice = null;
    _notify();
  }

  void dismissNotice() {
    notice = null;
    _notify();
  }

  void showToast(String text) {
    toast = text;
    _notify();
  }

  void clearToast() {
    if (toast.isEmpty) return;
    toast = '';
    _notify();
  }

  // ── 登录 ────────────────────────────────────────────────────────

  Future<void> refreshCaptcha() => _run(() async {
    captcha = await rust.fetchCaptcha();
    captchaInput.clear();
  });

  Future<void> fillCaptchaByOcr() => _run(() async {
    captcha ??= await rust.fetchCaptcha();
    final text = await rust.ocrCaptcha();
    final found = RegExp(r'[A-Za-z0-9]{4}').stringMatch(text) ?? text;
    captchaInput.text = found.toUpperCase();
  });

  Future<void> signIn() => _run(() async {
    if (loginname.trim().isEmpty) throw StateError('先填学号');
    if (password.isEmpty) throw StateError('先填密码');
    final LoginView view;
    if (autoOcr) {
      view = await rust.loginAuto(loginname: loginname.trim(), password: password);
    } else {
      final code = captchaInput.text.trim();
      if (code.isEmpty) throw StateError('先填验证码，或点「换一张」拿一张新的');
      captcha ??= await rust.fetchCaptcha();
      view = await rust.login(
        loginname: loginname.trim(),
        password: password,
        code: code,
      );
    }
    loggedIn = true;
    loginMessage = view.message;
    batches = view.batches;
    batchIndex = batches.indexWhere((batch) => batch.open);
    if (batchIndex < 0) batchIndex = batches.isEmpty ? -1 : 0;
    if (batches.isEmpty) {
      notice = '登录成功，但这个账号没有可选批次';
    }
  });

  Future<void> enterBatch(int index) => _run(() async {
    batchIndex = index;
    classTypes = await rust.openBatch(index: index);
    classType = classTypes.isEmpty ? null : classTypes.first;
    picked.clear();
    courses = [];
    if (classType != null) {
      await _loadCourses();
    }
    section = 2;
  });

  Future<void> selectClassType(String type) => _run(() async {
    classType = type;
    await _loadCourses();
  });

  Future<void> reloadCourses() => _run(_loadCourses);

  Future<void> _loadCourses() async {
    final type = classType;
    if (type == null) return;
    courses = await rust.listClasses(classType: type);
    picked.clear();
  }

  // ── 选课 / 退课 ─────────────────────────────────────────────────

  void togglePick(CourseRow row) {
    final key = _key(row);
    if (!picked.remove(key)) picked.add(key);
    _notify();
  }

  void pickAllVisible() {
    for (final row in visibleCourses) {
      picked.add(_key(row));
    }
    _notify();
  }

  void clearPicked() {
    picked.clear();
    _notify();
  }

  Future<void> submitPicked(String action) => _run(() async {
    final rows = pickedRows;
    if (rows.isEmpty) throw StateError('先勾几门课');
    final results = await rust.submit(rows: rows, action: action, continuous: autoRetry);
    _reportResults(results, action);
    if (action == 'del') {
      await _loadCourses();
    }
  });

  Future<void> runAllTasks() => _run(() async {
    if (tasks.isEmpty) throw StateError('任务列表是空的，先在下面加一条');
    running = true;
    _notify();
    try {
      final results = await rust.runTasks(tasks: tasks, continuous: autoRetry);
      _reportResults(results, 'add');
    } finally {
      running = false;
    }
  });

  void stopRun() {
    rust.stop();
    showToast('已发出停止信号，重试会在下一次请求前退出');
  }

  void _reportResults(List<SubmitResult> results, String action) {
    final ok = results.where((result) => result.ok).length;
    final label = action == 'del' ? '退课' : '选课';
    final failed = results.where((result) => !result.ok).toList();
    toast = '$label完成 $ok / ${results.length}'
        '${failed.isEmpty ? '' : '，失败 ${failed.length} 门'}';
    if (failed.isNotEmpty) {
      notice = failed
          .take(3)
          .map((result) => '${result.kch}-${result.kxh}：${result.message}')
          .join('\n');
    }
  }

  // ── 诊断 ────────────────────────────────────────────────────────

  Future<void> runInspect() => _run(() async {
    final kchs = tasks.map((task) => task.kch).where((kch) => kch.trim().isNotEmpty).toSet().toList();
    if (kchs.isEmpty) throw StateError('先在任务里填课程号，或到「课程」里拉一次列表');
    inspectLines = await rust.inspect(kchs: kchs);
  });

  // ── 任务编辑 ────────────────────────────────────────────────────

  void addTask() {
    final kch = taskKch.text.trim();
    final kxh = taskKxh.text.trim();
    if (kch.isEmpty || kxh.isEmpty) {
      notice = '课程号（KCH）和课序号（KXH）都要填';
      _notify();
      return;
    }
    tasks = [
      ...tasks,
      TaskView(
        category: taskCategory,
        action: taskAction,
        kch: kch,
        kxh: kxh,
        kcm: taskKcm.text.trim(),
      ),
    ];
    taskKch.clear();
    taskKxh.clear();
    taskKcm.clear();
    _notify();
  }

  void addPickedAsTasks() {
    final rows = pickedRows;
    if (rows.isEmpty) {
      notice = '先在课程列表里勾几门课';
      _notify();
      return;
    }
    final added = <TaskView>[];
    for (final row in rows) {
      final exists = tasks.any((task) => task.kch == row.kch && task.kxh == row.kxh);
      if (exists) continue;
      added.add(TaskView(
        category: row.classType.contains('BX') ? 0 : 1,
        action: 'add',
        kch: row.kch,
        kxh: row.kxh,
        kcm: row.kcm,
      ));
    }
    tasks = [...tasks, ...added];
    toast = added.isEmpty ? '勾的课都已经在任务里了' : '已加入 ${added.length} 条任务';
    _notify();
  }

  void removeTask(int index) {
    tasks = [...tasks]..removeAt(index);
    _notify();
  }

  void setTaskAction(int index, String action) {
    final task = tasks[index];
    tasks = [...tasks]..[index] = TaskView(
        category: task.category,
        action: action,
        kch: task.kch,
        kxh: task.kxh,
        kcm: task.kcm,
      );
    _notify();
  }

  // ── 设置 ────────────────────────────────────────────────────────

  void setAutoOcr(bool value) {
    autoOcr = value;
    _notify();
  }

  void setAutoRetry(bool value) {
    autoRetry = value;
    _notify();
  }

  void setCourseQuery(String value) {
    // 文本已经在 controller 里，这里只负责让筛选结果刷新
    _notify();
  }

  void setTaskCategory(int value) {
    taskCategory = value;
    _notify();
  }

  void toggleOnlyProblems() {
    onlyProblems = !onlyProblems;
    _notify();
  }

  void clearLogs() {
    logs.clear();
    _notify();
  }

  Future<void> saveConfig() => _run(() async {
    final path = bridge.saveConfig(
      view: ConfView(
        path: conf.path,
        exists: conf.exists,
        loginname: loginname.trim(),
        password: password,
        batch: conf.batch,
        ocrCaptcha: autoOcr,
        always: autoRetry,
        intervalMs: conf.intervalMs,
        tasks: tasks,
        note: '',
      ),
    );
    confPath = path;
    conf = bridge.loadConfig();
    toast = '已写入 $path';
  });

  // ── 配置文件位置 ────────────────────────────────────────────────

  static const _jsonGroup = XTypeGroup(label: '配置文件', extensions: ['json']);

  /// 挑一个已有的 conf.json 用。
  Future<void> pickConfFile() async {
    if (confLocked || busy) return;
    final file = await openFile(acceptedTypeGroups: const [_jsonGroup]);
    if (file == null) return;
    await _switchConfTo(file.path);
  }

  /// 另存到一个新位置（选好路径后当场写一份当前内容）。
  Future<void> saveConfAs() async {
    if (confLocked || busy) return;
    final location = await getSaveLocation(
      suggestedName: 'conf.json',
      acceptedTypeGroups: const [_jsonGroup],
    );
    if (location == null) return;
    await _switchConfTo(location.path, writeNow: true);
  }

  Future<void> _switchConfTo(String path, {bool writeNow = false}) => _run(() async {
        final actual = paths.setConfPath(path: path);
        confPath = actual;
        if (writeNow) {
          bridge.saveConfig(
            view: ConfView(
              path: actual,
              exists: true,
              loginname: loginname.trim(),
              password: password,
              batch: conf.batch,
              ocrCaptcha: autoOcr,
              always: autoRetry,
              intervalMs: conf.intervalMs,
              tasks: tasks,
              note: '',
            ),
          );
        }
        _reloadConfig();
        toast = writeNow ? '已另存到 $actual' : '配置文件已切换到 $actual';
      });

  /// 回到默认位置（%APPDATA%\xdxk-gui\conf.json）。
  Future<void> useDefaultConfPath() => _run(() async {
        final actual = paths.resetConfPath();
        confPath = actual;
        _reloadConfig();
        toast = '已恢复默认位置 $actual';
      });

  /// 在资源管理器里定位这个文件。
  Future<void> revealConfFile() => _run(() async {
        paths.revealConf();
        toast = '已在资源管理器里定位';
      });

  /// 换文件后把界面上的表单重置成新文件的内容。
  void _reloadConfig() {
    conf = bridge.loadConfig();
    loginnameInput.text = conf.loginname;
    passwordInput.text = conf.password;
    autoOcr = conf.ocrCaptcha;
    autoRetry = conf.always;
    tasks = List.of(conf.tasks);
    if (conf.note.isNotEmpty) notice = conf.note;
    _notify();
  }
}
