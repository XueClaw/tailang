import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'screens/workbench_screen.dart';
import 'state/workbench_state.dart';

void main() {
  runApp(const TailangGuiApp());
}

class TailangGuiApp extends StatelessWidget {
  const TailangGuiApp({super.key});

  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (_) => WorkbenchState(),
      child: MaterialApp(
        title: 'Tailang GUI',
        debugShowCheckedModeBanner: false,
        theme: ThemeData(
          colorScheme: ColorScheme.fromSeed(
            seedColor: const Color(0xFF0F766E),
            brightness: Brightness.light,
          ),
          scaffoldBackgroundColor: const Color(0xFFF3F4F6),
          useMaterial3: true,
        ),
        home: const WorkbenchScreen(),
      ),
    );
  }
}
