// 输入框：和 ef 套件的 EfField 同一副样子，但由外部 controller 驱动，
// 因为登录 / 任务表单需要读值、清空、预填。
import 'package:flutter/material.dart';

import '../../ef/tokens.dart';

class EfInput extends StatelessWidget {
  const EfInput({
    super.key,
    required this.label,
    required this.controller,
    this.hint,
    this.help,
    this.obscure = false,
    this.enabled = true,
    this.maxLength,
    this.monospaceDigits = false,
    this.onSubmitted,
    this.onChanged,
  });

  final String label;
  final TextEditingController controller;
  final String? hint;
  final String? help;
  final bool obscure;
  final bool enabled;
  final int? maxLength;
  final bool monospaceDigits;
  final ValueChanged<String>? onSubmitted;
  final ValueChanged<String>? onChanged;

  @override
  Widget build(BuildContext context) {
    final c = EfColors.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: EfType.ui(13, weight: 600, color: c.ink2)),
        const SizedBox(height: 6),
        TextField(
          controller: controller,
          obscureText: obscure,
          enabled: enabled,
          maxLength: maxLength,
          onSubmitted: onSubmitted,
          onChanged: onChanged,
          style: EfType.ui(16, width: monospaceDigits ? 112 : 100, color: c.ink),
          decoration: InputDecoration(
            isDense: true,
            counterText: '',
            filled: true,
            fillColor: c.panel,
            hintText: hint,
            hintStyle: EfType.ui(16, color: c.ink2),
            contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 13),
            enabledBorder: OutlineInputBorder(
              borderRadius: BorderRadius.zero,
              borderSide: BorderSide(color: c.rule),
            ),
            disabledBorder: OutlineInputBorder(
              borderRadius: BorderRadius.zero,
              borderSide: BorderSide(color: c.rule),
            ),
            focusedBorder: UnderlineInputBorder(
              borderRadius: BorderRadius.zero,
              borderSide: BorderSide(color: c.trace, width: 2),
            ),
          ),
        ),
        if (help != null) ...[
          const SizedBox(height: 6),
          Text(help!, style: EfType.ui(12, color: c.ink2)),
        ],
      ],
    );
  }
}
