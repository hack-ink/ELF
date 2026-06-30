use crate::RealWorldJob;

pub(in crate::operational) fn job_has_tag(job: &RealWorldJob, tag: &str) -> bool {
	job.tags.iter().any(|candidate| candidate == tag)
}
