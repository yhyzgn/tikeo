package net.tikeo.management.client;

import net.tikeo.management.model.CreateJobRequest;
import net.tikeo.management.model.JobDefinition;
import net.tikeo.management.model.JobInstance;
import net.tikeo.management.model.TriggerJobRequest;
import net.tikeo.management.model.UpdateJobRequest;
import java.util.List;

/** Control-plane client for managing jobs in a namespace/app scope. */
public interface TikeoJobClient {
    /** List jobs visible in the configured namespace/app scope. */
    List<JobDefinition> listJobs();

    /** Create a job in the configured namespace/app scope. */
    JobDefinition createJob(CreateJobRequest request);

    /** Update an existing job. */
    JobDefinition updateJob(String jobId, UpdateJobRequest request);

    /** Enable an existing job. */
    default JobDefinition enableJob(String jobId) {
        return updateJob(jobId, UpdateJobRequest.enable());
    }

    /** Disable an existing job. */
    default JobDefinition disableJob(String jobId) {
        return updateJob(jobId, UpdateJobRequest.disable());
    }

    /** Delete an existing job. */
    void deleteJob(String jobId);

    /** Trigger an existing job. */
    JobInstance triggerJob(String jobId, TriggerJobRequest request);
}
