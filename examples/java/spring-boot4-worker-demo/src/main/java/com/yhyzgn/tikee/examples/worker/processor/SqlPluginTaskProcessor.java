package com.yhyzgn.tikee.examples.worker.processor;

import com.yhyzgn.tikee.processor.TikeeProcessor;
import com.yhyzgn.tikee.processor.TikeeProcessorKind;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Demo plugin-backed processor; value is executor name and pluginType is structured metadata. */
@Slf4j
@Component
public final class SqlPluginTaskProcessor {
    @TikeeProcessor(value = "billing.sql-sync", kind = TikeeProcessorKind.PLUGIN, pluginType = "sql")
    public String run(String payload) {
        log.info("[billing.sql-sync] plugin SQL processor received payload='{}'", payload);
        return "sql-plugin-ok:" + payload;
    }
}
