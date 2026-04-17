import 'package:flutter/material.dart';

class SourcePanel extends StatelessWidget {
  const SourcePanel({
    super.key,
    required this.title,
    required this.content,
    required this.selected,
  });

  final String title;
  final String content;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    return Container(
      color: selected ? const Color(0xFFFFFFFF) : const Color(0xFFF8FAFC),
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 12),
          Expanded(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: const Color(0xFF111827),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Align(
                  alignment: Alignment.topLeft,
                  child: Text(
                    content.isEmpty ? 'No content loaded.' : content,
                    style: const TextStyle(
                      color: Color(0xFFD1D5DB),
                      fontFamily: 'Consolas',
                      fontSize: 14,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
