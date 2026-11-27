// 命令行小工具，直接 print 进度就行。
// ignore_for_file: avoid_print

// 生成应用图标：把一张方图导出成多尺寸 .ico。
//
//   dart run tool/gen_icon.dart <源图路径> [--paper|--bleed] [--transparent]
//
// 产物：
//   windows/runner/resources/app_icon.ico   Windows 可执行文件图标（7 个尺寸）
//   assets/branding/app_icon.png            512px 主图，README / 商店页用
//
// 两种处理方式，默认自动判断：
//   --paper   白底产品图：从四边抠掉白底刷成纸色 #F4F4F2，裁到内容再补正方形留白。
//             抠白是为了不留白边，纸色正好是界面底色。
//   --bleed   满版插画（深色背景、有边框）：不抠不补白，只做居中裁方 + 缩放。
//             四角不是近白就自动走这一路。
//   --transparent  纸色模式下改为真透明背景。
import 'dart:io';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:image/image.dart' as img;

const _paper = [244, 244, 242]; // #F4F4F2
const _sizes = [256, 128, 64, 48, 32, 24, 16];

enum _Mode { auto, paper, bleed }

void main(List<String> args) async {
  final positional = args.where((arg) => !arg.startsWith('--')).toList();
  // 不带参数就用仓库里放着的图标源图
  final source = positional.isNotEmpty ? positional.first : 'assets/branding/xdu_frog.jpg';
  final mode = args.contains('--bleed')
      ? _Mode.bleed
      : args.contains('--paper')
          ? _Mode.paper
          : _Mode.auto;
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

  final resolved = mode == _Mode.auto ? _detect(decoded) : mode;
  print(resolved == _Mode.bleed ? '模式：满版（不抠底、不留白）' : '模式：白底 → 纸色');

  final canvas = resolved == _Mode.bleed
      ? _bleed(decoded)
      : _paperMode(decoded, transparent: transparent);
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

/// 四角都是近白 → 白底产品图；否则当满版插画。
_Mode _detect(img.Image source) {
  final work = source.convert(numChannels: 4);
  final corners = [
    (0, 0),
    (work.width - 1, 0),
    (0, work.height - 1),
    (work.width - 1, work.height - 1),
  ];
  var white = 0;
  for (final (x, y) in corners) {
    final p = work.getPixel(x, y);
    final minC = math.min(p.r.toInt(), math.min(p.g.toInt(), p.b.toInt()));
    final maxC = math.max(p.r.toInt(), math.max(p.g.toInt(), p.b.toInt()));
    if (minC >= 200 && maxC - minC <= 18) white++;
  }
  return white >= 3 ? _Mode.paper : _Mode.bleed;
}

/// 满版：居中裁成正方形，原样缩放，不留白也不改底色。
img.Image _bleed(img.Image source) {
  final work = source.convert(numChannels: 4);
  final side = math.min(work.width, work.height);
  return img.copyCrop(
    work,
    x: (work.width - side) ~/ 2,
    y: (work.height - side) ~/ 2,
    width: side,
    height: side,
  );
}

/// 白底：抠掉背景（刷纸色或透明）、裁到内容、补成正方形并留 12% 边距。
img.Image _paperMode(img.Image source, {required bool transparent}) {
  final work = source.convert(numChannels: 4);
  _cutBackground(work);

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
  if (!transparent) {
    img.fill(canvas, color: img.ColorRgba8(_paper[0], _paper[1], _paper[2], 255));
  }
  img.compositeImage(
    canvas,
    cropped,
    dstX: pad + (side - cropped.width) ~/ 2,
    dstY: pad + (side - cropped.height) ~/ 2,
  );
  return canvas;
}

/// 从四条边泛洪，把连通的白色（含浅灰投影）刷成透明。
/// 被主体包住的白（比如青蛙的白肚皮）不连通，不会被刷掉。
void _cutBackground(img.Image image) {
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
