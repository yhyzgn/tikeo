package net.tikeo.examples.worker.processor;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TikeoProcessor;
import net.tikeo.processor.TikeoProcessorKind;
import net.tikeo.processor.TikeoPluginType;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Demo plugin-backed processor; value is executor name and pluginType is structured metadata. */
@Slf4j
@Component
public final class SqlPluginTaskProcessor {
    @TikeoProcessor(value = "billing.sql-sync", kind = TikeoProcessorKind.PLUGIN, description = "SQL 同步插件执行器示例", pluginType = TikeoPluginType.SQL)
    public String run(TaskContext context, String payload) {
        log.info("[billing.sql-sync] plugin SQL processor received payload='{}'", payload);
        return "sql-plugin-ok:" + payload;
    }
}
