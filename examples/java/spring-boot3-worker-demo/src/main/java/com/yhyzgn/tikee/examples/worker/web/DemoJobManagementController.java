package com.yhyzgn.tikee.examples.worker.web;

import com.yhyzgn.tikee.management.client.TikeeJobClient;
import com.yhyzgn.tikee.management.model.CreateJobRequest;
import com.yhyzgn.tikee.management.model.JobDefinition;
import com.yhyzgn.tikee.management.model.JobInstance;
import com.yhyzgn.tikee.management.model.TriggerJobRequest;
import java.util.List;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/** Demo endpoints showing Java SDK job-management operations. */
@Slf4j
@RestController
@RequestMapping("/demo/jobs")
@RequiredArgsConstructor
@ConditionalOnProperty(prefix = "tikee.management", name = "enabled", havingValue = "true")
public final class DemoJobManagementController {
    private final TikeeJobClient jobClient;

    @GetMapping
    public List<JobDefinition> listJobs() {
        List<JobDefinition> jobs = jobClient.listJobs();
        log.info("[demo.management] listed {} jobs", jobs.size());
        return jobs;
    }

    @PostMapping("/echo")
    public ManagedJobExample createDisableEnableAndTriggerEchoJob() {
        JobDefinition created = jobClient.createJob(CreateJobRequest.api("demo managed echo", "demo.echo"));
        log.info("[demo.management] created job id={} processor={}", created.id(), created.processorName());
        JobDefinition disabled = jobClient.disableJob(created.id());
        log.info("[demo.management] disabled job id={}", disabled.id());
        JobDefinition enabled = jobClient.enableJob(created.id());
        log.info("[demo.management] enabled job id={}", enabled.id());
        JobInstance instance = jobClient.triggerJob(created.id(), TriggerJobRequest.api());
        log.info("[demo.management] triggered job id={} instance={}", created.id(), instance.id());
        return new ManagedJobExample(enabled, instance);
    }


    @PostMapping("/script/{scriptId}")
    public ManagedJobExample createAndTriggerScriptJob(@PathVariable String scriptId) {
        JobDefinition created = jobClient.createJob(CreateJobRequest.apiScript("demo managed script", scriptId));
        log.info("[demo.management] created script job id={} scriptId={}", created.id(), created.scriptId());
        JobInstance instance = jobClient.triggerJob(created.id(), TriggerJobRequest.api());
        log.info("[demo.management] triggered script job id={} instance={}", created.id(), instance.id());
        return new ManagedJobExample(created, instance);
    }

    @PostMapping("/plugin/sql")
    public ManagedJobExample createAndTriggerSqlPluginJob() {
        JobDefinition created = jobClient.createJob(CreateJobRequest.apiPlugin(
                "demo managed sql plugin",
                "sql",
                "billing.sql-sync"));
        log.info(
                "[demo.management] created plugin job id={} processorType={} processorName={}",
                created.id(),
                created.processorType(),
                created.processorName());
        JobInstance instance = jobClient.triggerJob(created.id(), TriggerJobRequest.api());
        log.info("[demo.management] triggered plugin job id={} instance={}", created.id(), instance.id());
        return new ManagedJobExample(created, instance);
    }

    public record ManagedJobExample(JobDefinition job, JobInstance instance) {}
}
