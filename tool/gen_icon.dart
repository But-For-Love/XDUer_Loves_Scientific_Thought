// 命令行小工具，直接 print 进度就行。
// ignore_for_file: avoid_print

// 生成应用图标：把一张白底产品图裁成方形、背景换成纸色，再导出多尺寸 .ico。
//
//   dart run tool/gen_icon.dart [源图路径] [--transparent]
//
// 产物：
//   windows/runner/resources/app_icon.ico   Windows 可执行文件图标（7 个尺寸）
//   assets/branding/app_icon.png            512px 主图，README / 商店页用
//
// 背景为什么用纸色而不是透明：原图带柔和投影，抠白容易留白边；纸色 #F4F4F2
// 正好是界面底色，贴到任务栏上像一张纸卡，也和 endfield-ui 的基调一致。
// 想要透明背景就加 --transparent。
import 'dart:io';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:image/image.dart' as img;

const _paper = [244, 244, 242]; // #F4F4F2
const _sizes = [256, 128, 64, 48, 32, 24, 16];

void main(List<String> args) async {
  final positional = args.where((arg) => !arg.startsWith('--')).toList();
  final source =
      positional.isNotEmpty ? positional.first : r'C:\Users\rog\Downloads\1794551956436.png';
  final transparent = args.contains('--transparent');

  final file = File(source);
  if (!file.existsSync()) {
    stderr.writeln('找不到源图：$source');
    exit(1);
  }
  final decoded = img.decodeImage(await file.readAsBytes());
  if (decoded == null) {
    stderr.writeln('解不开这张图：$source');
    exit(1);
  }
  print('源图 ${decoded.width}×${decoded.height}');

  final canvas = _prepare(decoded, transparent: transparent);
  print('画布 ${canvas.width}×${canvas.height}');

  final masterDir = Directory('assets/branding')..createSync(recursive: true);
  final master = img.copyResize(canvas, width: 512, interpolation: img.Interpolation.cubic);
  File('${masterDir.path}/app_icon.png').writeAsBytesSync(img.encodePng(master));
  print('写出 ${masterDir.path}/app_icon.png');

  final frames = [
    for (final size in _sizes)
      img.copyResize(
        canvas,
        width: size,
        height: size,
        interpolation: size <= 32 ? img.Interpolation.average : img.Interpolation.cubic,
      ),
  ];
  final ico = img.IcoEncoder().encodeImages(frames);
  const icoPath = 'windows/runner/resources/app_icon.ico';
  File(icoPath).writeAsBytesSync(ico);
  print('写出 $icoPath（${_sizes.join(' / ')}，${ico.length} 字节）');

  final previewDir = Directory('build/icon-preview')..createSync(recursive: true);
  for (final frame in frames) {
    File('${previewDir.path}/app_icon_${frame.width}.png').writeAsBytesSync(img.encodePng(frame));
  }
  print('预览 ${previewDir.path}/');
}

img.Image _prepare(img.Image source, {required bool transparent}) {
  final work = source.convert(numChannels: 4);
  // 先统一按透明处理：抠完再决定是铺纸色还是留透明
  _cutBackground(work, transparent: true);

  // 内容边界
  var minX = work.width, minY = work.height, maxX = -1, maxY = -1;
  for (final pixel in work) {
    if (pixel.a < 8) continue;
    minX = math.min(minX, pixel.x);
    minY = math.min(minY, pixel.y);
    maxX = math.max(maxX, pixel.x);
    maxY = math.max(maxY, pixel.y);
  }
  if (maxX < 0) throw StateError('没找到内容，整张图都是背景色？');

  final cropped = img.copyCrop(
    work,
    x: minX,
    y: minY,
    width: maxX - minX + 1,
    height: maxY - minY + 1,
  );
  print('内容 ${cropped.width}×${cropped.height}');

  final side = math.max(cropped.width, cropped.height);
  final pad = (side * 0.12).round();
  final edge = side + pad * 2;
  final canvas = img.Image(width: edge, height: edge, numChannels: 4);
  final paper = img.ColorRgba8(_paper[0], _paper[1], _paper[2], 255);
  if (!transparent) {
    img.fill(canvas, color: paper);
  }
  img.compositeImage(
    canvas,
    cropped,
    dstX: pad + (side - cropped.width) ~/ 2,
    dstY: pad + (side - cropped.height) ~/ 2,
  );
  return canvas;
}

/// 从四条边泛洪，把连通的白色（含浅灰投影）刷掉：纸色模式刷成纸色，透明模式刷成 a=0。
/// 青蛙身体包住的白肚皮因为不连通，不会被刷掉。
void _cutBackground(img.Image image, {required bool transparent}) {
  final width = image.width;
  final height = image.height;
  final total = width * height;
  final visited = Uint8List(total);
  final stack = Int32List(total);
  var top = 0;

  bool isBackground(int index) {
    final pixel = image.getPixel(index % width, index ~/ width);
    final r = pixel.r.toInt();
    final g = pixel.g.toInt();
    final b = pixel.b.toInt();
    final minC = math.min(r, math.min(g, b));
    final maxC = math.max(r, math.max(g, b));
    // 亮且几乎无彩色的算背景：白色底 + 浅灰投影
    return minC >= 196 && (maxC - minC) <= 18;
  }

  void push(int index) {
    if (visited[index] != 0) return;
    if (!isBackground(index)) return;
    visited[index] = 1;
    stack[top++] = index;
  }

  for (var x = 0; x < width; x++) {
    push(x);
    push((height - 1) * width + x);
  }
  for (var y = 0; y < height; y++) {
    push(y * width);
    push(y * width + width - 1);
  }

  final fill = img.ColorRgba8(0, 0, 0, 0);

  var cleared = 0;
  while (top > 0) {
    final index = stack[--top];
    final x = index % width;
    final y = index ~/ width;
    image.setPixel(x, y, fill);
    cleared++;
    if (x > 0) push(index - 1);
    if (x < width - 1) push(index + 1);
    if (y > 0) push(index - width);
    if (y < height - 1) push(index + width);
  }
  print('背景像素 $cleared / $total');
}
